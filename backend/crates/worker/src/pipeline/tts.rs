use anyhow::Result;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;
use tracing::{info, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordTimestamp {
    pub word: String,
    pub start: f64,
    pub end: f64,
}

pub struct TtsEngine {
    pub temp_dir: PathBuf,
    pub elevenlabs_key: String,
    pub deepgram_key: String,
}

impl TtsEngine {
    pub fn new(temp_dir: PathBuf, elevenlabs_key: String, deepgram_key: String) -> Self {
        Self { temp_dir, elevenlabs_key, deepgram_key }
    }

    /// Synthesizes speech audio for a text sentence chunk
    pub async fn synthesize(&self, text: &str, idx: usize, provider: &str, voice: &str) -> Result<PathBuf> {
        let output_file = self.temp_dir.join(format!("studio_speech_{}.mp3", idx));
        info!("Synthesizing TTS with provider '{}', voice '{}' -> {:?}", provider, voice, output_file);

        match provider {
            "elevenlabs" => self.synthesize_elevenlabs(text, voice, &output_file).await?,
            "deepgram-aura" => self.synthesize_deepgram_aura(text, voice, &output_file).await?,
            _ => self.synthesize_edge_tts(text, voice, &output_file).await?,
        }

        Ok(output_file)
    }

    /// Synthesizes the full script as one audio and returns word-level timestamps
    pub async fn synthesize_full(&self, text: &str, provider: &str, voice: &str) -> Result<(PathBuf, Vec<WordTimestamp>)> {
        let audio_path = self.temp_dir.join("full_voiceover.mp3");
        info!("Synthesizing full-script TTS ({}) using provider '{}'", text.len(), provider);

        let timestamps = match provider {
            "elevenlabs" => self.synthesize_elevenlabs_full(text, voice, &audio_path).await?,
            "deepgram-aura" => {
                self.synthesize_deepgram_aura(text, voice, &audio_path).await?;
                if !self.deepgram_key.is_empty() {
                    self.transcribe_for_timestamps(&audio_path).await?
                } else {
                    self.estimate_word_timestamps(text, &audio_path).await
                }
            }
            _ => {
                self.synthesize_edge_tts(text, voice, &audio_path).await?;
                self.estimate_word_timestamps(text, &audio_path).await
            }
        };

        info!("TTS audio written to {:?}", audio_path);
        Ok((audio_path, timestamps))
    }

    /// Estimate word timestamps evenly spaced across the audio duration
    async fn estimate_word_timestamps(&self, text: &str, audio_path: &Path) -> Vec<WordTimestamp> {
        let duration = get_audio_duration_simple(audio_path).await.unwrap_or(5.0);
        let words: Vec<&str> = text.split_whitespace().collect();
        let n = words.len().max(1) as f64;
        let word_dur = duration / n;
        words.iter().enumerate().map(|(i, w)| WordTimestamp {
            word: w.to_string(),
            start: i as f64 * word_dur,
            end: (i as f64 + 1.0) * word_dur,
        }).collect()
    }

    async fn transcribe_for_timestamps(&self, audio_path: &Path) -> Result<Vec<WordTimestamp>> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()?;
        let audio_bytes = tokio::fs::read(audio_path).await?;
        let ext = audio_path.extension().and_then(|e| e.to_str()).unwrap_or("mp3");
        let content_type = match ext {
            "wav" => "audio/wav",
            "m4a" => "audio/mp4",
            "ogg" => "audio/ogg",
            _ => "audio/mpeg",
        };

        let resp: Value = client.post("https://api.deepgram.com/v1/listen?model=nova-2&punctuate=true&utterances=false&paragraphs=false")
            .header("Authorization", format!("Token {}", self.deepgram_key))
            .header("Content-Type", content_type)
            .body(audio_bytes)
            .send()
            .await?
            .json()
            .await?;

        let words = resp["results"]["channels"][0]["alternatives"][0]["words"]
            .as_array()
            .map(|arr| {
                arr.iter().filter_map(|w| {
                    Some(WordTimestamp {
                        word: w["word"].as_str()?.to_string(),
                        start: w["start"].as_f64()?,
                        end: w["end"].as_f64()?,
                    })
                }).collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if words.is_empty() {
            anyhow::bail!("Deepgram STT returned no words");
        }
        info!("Deepgram STT returned {} word timestamps", words.len());
        Ok(words)
    }

    async fn synthesize_edge_tts(&self, text: &str, voice: &str, output: &Path) -> Result<()> {
        let v = if voice.is_empty() { "en-US-ChristopherNeural" } else { voice };
        let out_str = output.to_string_lossy().to_string();

        // Try python -m edge_tts (works with venv)
        let status = Command::new("python")
            .arg("-m").arg("edge_tts")
            .arg("--text").arg(text)
            .arg("--voice").arg(v)
            .arg("--write-media").arg(&out_str)
            .status().await;

        if let Ok(s) = status {
            if s.success() {
                return Ok(());
            }
        }

        // Try edge-tts directly (if in PATH)
        let status = Command::new("edge-tts")
            .arg("--text").arg(text)
            .arg("--voice").arg(v)
            .arg("--write-media").arg(&out_str)
            .status().await;

        if let Ok(s) = status {
            if s.success() {
                return Ok(());
            }
        }

        // Try venv python directly
        let venv_edge = Path::new("novaclip_reframe").join("venv").join("Scripts").join("edge-tts.exe");
        if venv_edge.exists() {
            let status = Command::new(&venv_edge)
                .arg("--text").arg(text)
                .arg("--voice").arg(v)
                .arg("--write-media").arg(&out_str)
                .status().await;
            if let Ok(s) = status {
                if s.success() {
                    return Ok(());
                }
            }
        }

        // Fallback to gTTS via Python
        info!("Edge-TTS fallback to python gTTS");
        let py_code = format!(
            "import sys; from gtts import gTTS; tts = gTTS(sys.argv[1]); tts.save(sys.argv[2])"
        );
        let _ = Command::new("python").arg("-c").arg(&py_code).arg(text).arg(&out_str).status().await;
        Ok(())
    }

    async fn synthesize_elevenlabs(&self, text: &str, voice_id: &str, output: &Path) -> Result<()> {
        if self.elevenlabs_key.is_empty() {
            anyhow::bail!("ElevenLabs API key is missing");
        }
        let vid = if voice_id.is_empty() || voice_id.len() < 10 { "21m00Tcm4TlvDq8ikWAM" } else { voice_id };
        let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{}", vid);

        let body = serde_json::json!({
            "text": text,
            "model_id": "eleven_monolingual_v1",
            "voice_settings": {"stability": 0.5, "similarity_boost": 0.5}
        });

        let client = reqwest::Client::new();
        let resp = client.post(&url)
            .header("xi-api-key", &self.elevenlabs_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if resp.status().is_success() {
            let bytes = resp.bytes().await?;
            tokio::fs::write(output, bytes).await?;
            Ok(())
        } else {
            anyhow::bail!("ElevenLabs TTS failed: {}", resp.text().await?);
        }
    }

    /// Full TTS via ElevenLabs /with-timestamps endpoint — returns native word timestamps
    async fn synthesize_elevenlabs_full(&self, text: &str, voice_id: &str, output: &Path) -> Result<Vec<WordTimestamp>> {
        if self.elevenlabs_key.is_empty() {
            anyhow::bail!("ElevenLabs API key is missing");
        }
        let vid = if voice_id.is_empty() || voice_id.len() < 10 { "21m00Tcm4TlvDq8ikWAM" } else { voice_id };
        let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{}/with-timestamps", vid);

        let body = serde_json::json!({
            "text": text,
            "model_id": "eleven_multilingual_v2",
            "voice_settings": {"stability": 0.5, "similarity_boost": 0.75}
        });

        let client = reqwest::Client::new();
        let resp = client.post(&url)
            .header("xi-api-key", &self.elevenlabs_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("ElevenLabs TTS with timestamps failed: {}", resp.text().await?);
        }

        let data: serde_json::Value = resp.json().await?;

        // Decode base64 audio
        let audio_b64 = data["audio_base64"].as_str()
            .ok_or_else(|| anyhow::anyhow!("ElevenLabs response missing audio_base64"))?;
        use base64::Engine;
        let audio_bytes = base64::engine::general_purpose::STANDARD.decode(audio_b64)?;
        tokio::fs::write(output, &audio_bytes).await?;
        info!("ElevenLabs TTS full saved: {} bytes", audio_bytes.len());

        // Extract character alignment
        let alignment = data["alignment"].as_object()
            .or_else(|| data["normalized_alignment"].as_object())
            .ok_or_else(|| anyhow::anyhow!("ElevenLabs response missing alignment data"))?;

        let chars = alignment["characters"].as_array()
            .and_then(|a| a.iter().map(|v| v.as_str().map(|s| s.to_string())).collect::<Option<Vec<_>>>())
            .ok_or_else(|| anyhow::anyhow!("ElevenLabs alignment missing characters"))?;
        let char_starts = alignment["character_start_times_seconds"].as_array()
            .and_then(|a| a.iter().map(|v| v.as_f64()).collect::<Option<Vec<_>>>())
            .ok_or_else(|| anyhow::anyhow!("ElevenLabs alignment missing start times"))?;
        let char_ends = alignment["character_end_times_seconds"].as_array()
            .and_then(|a| a.iter().map(|v| v.as_f64()).collect::<Option<Vec<_>>>())
            .ok_or_else(|| anyhow::anyhow!("ElevenLabs alignment missing end times"))?;

        // Convert character-level to word-level timestamps
        let mut words: Vec<WordTimestamp> = Vec::new();
        let mut cur = String::new();
        let mut cur_start: Option<f64> = None;
        for i in 0..chars.len() {
            let ch = &chars[i];
            let s = char_starts.get(i).copied().unwrap_or(0.0);
            let e = char_ends.get(i).copied().unwrap_or(0.0);
            if ch.trim().is_empty() {
                if !cur.is_empty() {
                    if let Some(cs) = cur_start {
                        words.push(WordTimestamp { word: cur.clone(), start: cs, end: e });
                    }
                    cur.clear();
                    cur_start = None;
                }
            } else {
                if cur_start.is_none() {
                    cur_start = Some(s);
                }
                cur.push_str(ch);
            }
        }
        if !cur.is_empty() {
            if let Some(cs) = cur_start {
                let last_end = char_ends.last().copied().unwrap_or(cs + 1.0);
                words.push(WordTimestamp { word: cur, start: cs, end: last_end });
            }
        }

        info!("ElevenLabs returned {} word timestamps", words.len());
        Ok(words)
    }

    async fn synthesize_deepgram_aura(&self, text: &str, voice: &str, output: &Path) -> Result<()> {
        if self.deepgram_key.is_empty() {
            anyhow::bail!("Deepgram API key is missing");
        }
        let valid_voices = ["aura-2-asteria-en", "aura-2-athena-en", "aura-2-luna-en", "aura-2-stella-en",
                            "aura-2-hera-en", "aura-2-orion-en", "aura-2-arcas-en", "aura-2-perseus-en",
                            "aura-2-angus-en", "aura-2-orpheus-en",
                            "aura-asteria-en", "aura-athena-en", "aura-luna-en", "aura-stella-en",
                            "aura-hera-en", "aura-orion-en", "aura-arcas-en", "aura-perseus-en",
                            "aura-angus-en", "aura-orpheus-en"];
        let model = if valid_voices.contains(&voice) { voice } else { "aura-asteria-en" };
        let url = format!("https://api.deepgram.com/v1/speak?model={}", model);

        let body = serde_json::json!({ "text": text });

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()?;
        let resp = client.post(&url)
            .header("Authorization", format!("Token {}", self.deepgram_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if resp.status().is_success() {
            let bytes = resp.bytes().await?;
            info!("Deepgram Aura TTS succeeded, {} bytes written", bytes.len());
            tokio::fs::write(output, &bytes).await?;
            Ok(())
        } else {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            warn!("Deepgram Aura TTS failed: HTTP {} {}", status, text);
            anyhow::bail!("Deepgram Aura TTS failed (HTTP {}): {}", status, text);
        }
    }
}

/// Get audio duration in seconds via ffprobe (simple helper)
async fn get_audio_duration_simple(path: &Path) -> Option<f64> {
    let output = Command::new("ffprobe")
        .args(["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0"])
        .arg(path.to_str()?)
        .output().await.ok()?;
    let s = std::str::from_utf8(&output.stdout).ok()?.trim().to_string();
    s.parse::<f64>().ok()
}
