use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::info;

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

    async fn synthesize_edge_tts(&self, text: &str, voice: &str, output: &Path) -> Result<()> {
        let v = if voice.is_empty() { "en-US-ChristopherNeural" } else { voice };
        
        // Execute edge-tts CLI tool asynchronously via std process
        let status = Command::new("edge-tts")
            .arg("--text")
            .arg(text)
            .arg("--voice")
            .arg(v)
            .arg("--write-media")
            .arg(output)
            .status();

        match status {
            Ok(s) if s.success() => Ok(()),
            _ => {
                // Fallback to gTTS via Python if edge-tts CLI is not installed
                info!("Edge-TTS fallback to python gTTS");
                let py_code = format!(
                    "from gtts import gTTS; tts = gTTS('{}'); tts.save('{}')",
                    text.replace("'", "\\'"), output.to_string_lossy().replace("\\", "/")
                );
                let _ = Command::new("python").arg("-c").arg(py_code).status();
                Ok(())
            }
        }
    }

    async fn synthesize_elevenlabs(&self, text: &str, voice_id: &str, output: &Path) -> Result<()> {
        if self.elevenlabs_key.is_empty() {
            anyhow::bail!("ElevenLabs API key is missing");
        }
        let vid = if voice_id.is_empty() { "21m00Tcm4TlvDq8ikWAM" } else { voice_id };
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

    async fn synthesize_deepgram_aura(&self, text: &str, voice: &str, output: &Path) -> Result<()> {
        if self.deepgram_key.is_empty() {
            anyhow::bail!("Deepgram API key is missing");
        }
        let model = if voice.is_empty() { "aura-asteria-en" } else { voice };
        let url = format!("https://api.deepgram.com/v1/speak?model={}", model);

        let body = serde_json::json!({ "text": text });

        let client = reqwest::Client::new();
        let resp = client.post(&url)
            .header("Authorization", format!("Token {}", self.deepgram_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if resp.status().is_success() {
            let bytes = resp.bytes().await?;
            tokio::fs::write(output, bytes).await?;
            Ok(())
        } else {
            anyhow::bail!("Deepgram Aura TTS failed: {}", resp.text().await?);
        }
    }
}
