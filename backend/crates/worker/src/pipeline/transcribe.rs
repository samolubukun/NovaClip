use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepgramWord {
    pub word: String,
    pub start: f64,
    pub end: f64,
    pub confidence: f64,
    pub punctuated_word: Option<String>,
    pub speaker: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerSegment {
    pub start: f64,
    pub end: f64,
    pub speaker: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimestampedTranscript {
    pub full_text: String,
    pub words: Vec<DeepgramWord>,
    pub duration: f64,
}

/// Format seconds to MM:SS
pub fn seconds_to_mmss(secs: f64) -> String {
    let total = secs as u64;
    let m = total / 60;
    let s = total % 60;
    format!("{:02}:{:02}", m, s)
}

/// Build chunked transcript lines with timestamps for the LLM prompt
pub fn build_transcript_for_prompt(transcript: &TimestampedTranscript) -> String {
    if transcript.words.is_empty() {
        return transcript.full_text.clone();
    }

    let mut lines = Vec::new();
    let mut chunk_words: Vec<&DeepgramWord> = Vec::new();
    let words_per_chunk = 40;

    for (i, word) in transcript.words.iter().enumerate() {
        chunk_words.push(word);
        if chunk_words.len() >= words_per_chunk || i == transcript.words.len() - 1 {
            let start = seconds_to_mmss(chunk_words.first().unwrap().start);
            let end = seconds_to_mmss(chunk_words.last().unwrap().end);
            let text: String = chunk_words.iter()
                .map(|w| w.punctuated_word.as_deref().unwrap_or(&w.word))
                .collect::<Vec<_>>()
                .join(" ");
            lines.push(format!("[{} - {}] {}", start, end, text));
            chunk_words.clear();
        }
    }
    lines.join("\n")
}

/// Helper to decode audio to 16kHz mono s16le PCM via ffmpeg
async fn decode_pcm_s16le(audio_path: &Path) -> Result<Vec<i16>> {
    let output = tokio::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            audio_path.to_str().unwrap_or_default(),
            "-vn",
            "-ac",
            "1",
            "-ar",
            "16000",
            "-f",
            "s16le",
            "-",
        ])
        .output()
        .await
        .context("Failed to execute ffmpeg for s16le PCM extraction")?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ffmpeg s16le decode failed: {}", err);
    }

    let bytes = output.stdout;
    let mut pcm = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.chunks_exact(2) {
        pcm.push(i16::from_le_bytes([chunk[0], chunk[1]]));
    }
    Ok(pcm)
}

/// Helper to decode audio to 16kHz mono f32le PCM via ffmpeg
async fn decode_pcm_f32le(audio_path: &Path) -> Result<Vec<f32>> {
    let output = tokio::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            audio_path.to_str().unwrap_or_default(),
            "-vn",
            "-ac",
            "1",
            "-ar",
            "16000",
            "-f",
            "f32le",
            "-",
        ])
        .output()
        .await
        .context("Failed to execute ffmpeg for f32le PCM extraction")?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ffmpeg f32le decode failed: {}", err);
    }

    let bytes = output.stdout;
    let mut pcm = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        let sample_bytes = [chunk[0], chunk[1], chunk[2], chunk[3]];
        pcm.push(f32::from_le_bytes(sample_bytes));
    }
    Ok(pcm)
}


/// Transcribe audio locally using Vosk batch recognition
#[cfg(feature = "vosk")]
pub async fn transcribe_with_vosk(
    audio_path: &Path,
    model_path: &Path,
    seg_model: &Path,
    emb_model: &Path,
) -> Result<TimestampedTranscript> {
    info!(
        "Transcribing locally with Vosk: {} (model: {})",
        audio_path.display(),
        model_path.display()
    );

    let pcm = decode_pcm_s16le(audio_path).await?;
    let model_path_buf = model_path.to_path_buf();

    let transcript_res = tokio::task::spawn_blocking::<_, anyhow::Result<TimestampedTranscript>>(move || {
        let model_str = model_path_buf
            .to_str()
            .context("Invalid Vosk model path string")?;
        let model = vosk::Model::new(model_str)
            .ok_or_else(|| anyhow::anyhow!("Failed to load Vosk model from '{}'", model_str))?;

        let mut recognizer = vosk::Recognizer::new(&model, 16000.0)
            .ok_or_else(|| anyhow::anyhow!("Failed to create Vosk recognizer"))?;

        recognizer.set_words(true);

        let mut words = Vec::new();
        let chunk_size = 8000; // 0.5s at 16kHz

        for chunk in pcm.chunks(chunk_size) {
            if let Ok(vosk::DecodingState::Finalized) = recognizer.accept_waveform(chunk) {
                if let Some(single) = recognizer.result().single() {
                    for w in single.result {
                        words.push(DeepgramWord {
                            word: w.word.to_string(),
                            start: f64::from(w.start),
                            end: f64::from(w.end),
                            confidence: f64::from(w.conf),
                            punctuated_word: Some(w.word.to_string()),
                            speaker: None,
                        });
                    }
                }
            }
        }

        if let Some(single) = recognizer.final_result().single() {
            for w in single.result {
                words.push(DeepgramWord {
                    word: w.word.to_string(),
                    start: f64::from(w.start),
                    end: f64::from(w.end),
                    confidence: f64::from(w.conf),
                    punctuated_word: Some(w.word.to_string()),
                    speaker: None,
                });
            }
        }

        let full_text = words
            .iter()
            .map(|w| w.word.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let duration = words.last().map(|w| w.end).unwrap_or(0.0);

        info!(
            "Vosk transcription complete: {} words, {:.1}s",
            words.len(),
            duration
        );

        Ok(TimestampedTranscript {
            full_text,
            words,
            duration,
        })
    })
    .await
    .context("Vosk blocking task panicked")?;

    let mut transcript = transcript_res?;
    if !transcript.words.is_empty() {
        diarize_words_local(audio_path, &mut transcript.words, seg_model, emb_model).await.ok();
    }
    Ok(transcript)
}

#[cfg(not(feature = "vosk"))]
pub async fn transcribe_with_vosk(
    _audio_path: &Path,
    _model_path: &Path,
    _seg_model: &Path,
    _emb_model: &Path,
) -> Result<TimestampedTranscript> {
    anyhow::bail!("Vosk not compiled in this build (feature disabled for CI). Use Deepgram or Whisper.")
}

/// Transcribe audio locally using Whisper batch recognition
#[cfg(feature = "whisper")]
pub async fn transcribe_with_whisper(
    audio_path: &Path,
    model_path: &Path,
    seg_model: &Path,
    emb_model: &Path,
) -> Result<TimestampedTranscript> {
    info!(
        "Transcribing locally with Whisper: {} (model: {})",
        audio_path.display(),
        model_path.display()
    );

    let pcm_f32 = decode_pcm_f32le(audio_path).await?;
    let model_path_buf = model_path.to_path_buf();

    let transcript_res = tokio::task::spawn_blocking::<_, anyhow::Result<TimestampedTranscript>>(move || {
        let model_str = model_path_buf
            .to_str()
            .context("Invalid Whisper model path string")?;
        let ctx_params = whisper_rs::WhisperContextParameters::default();
        let context = whisper_rs::WhisperContext::new_with_params(model_str, ctx_params)
            .map_err(|e| anyhow::anyhow!("Failed to load Whisper model from '{}': {}", model_str, e))?;

        let mut params =
            whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(4);
        params.set_language(Some("en"));
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_token_timestamps(true);
        params.set_split_on_word(true);
        params.set_suppress_blank(true);
        params.set_suppress_nst(true);
        params.set_temperature(0.0);

        let mut state = context
            .create_state()
            .map_err(|e| anyhow::anyhow!("Failed to create Whisper state: {}", e))?;

        state
            .full(params, &pcm_f32)
            .map_err(|e| anyhow::anyhow!("Whisper inference failed: {}", e))?;

        let mut words = Vec::new();
        let mut full_text_parts = Vec::new();

        let segment_count = state
            .full_n_segments()
            .map_err(|e| anyhow::anyhow!("Whisper segment count error: {}", e))?;

        for i in 0..segment_count {
            let text = state
                .full_get_segment_text(i)
                .map_err(|e| anyhow::anyhow!("Whisper text error at segment {}: {}", i, e))?;
            let start = state.full_get_segment_t0(i).unwrap_or(0) as f64 / 100.0;
            let end = state.full_get_segment_t1(i).unwrap_or(0) as f64 / 100.0;

            let trimmed = text.trim();
            if !trimmed.is_empty() {
                full_text_parts.push(trimmed.to_string());

                let n_tokens = state.full_n_tokens(i).unwrap_or(0);
                let mut segment_prob = 0.0f32;
                let mut segment_token_count = 0;

                for j in 0..n_tokens {
                    if let Ok(token_data) = state.full_get_token_data(i, j) {
                        if token_data.id < context.token_eot() {
                            segment_prob += token_data.p;
                            segment_token_count += 1;
                        }
                    }
                }

                let confidence = if segment_token_count > 0 {
                    f64::from(segment_prob / segment_token_count as f32)
                } else {
                    1.0
                };

                words.push(DeepgramWord {
                    word: trimmed.to_lowercase(),
                    start,
                    end,
                    confidence,
                    punctuated_word: Some(trimmed.to_string()),
                    speaker: None,
                });
            }
        }

        let full_text = full_text_parts.join(" ");
        let duration = words.last().map(|w| w.end).unwrap_or(0.0);

        info!(
            "Whisper transcription complete: {} words, {:.1}s",
            words.len(),
            duration
        );

        Ok(TimestampedTranscript {
            full_text,
            words,
            duration,
        })
    })
    .await
    .context("Whisper blocking task panicked")?;

    let mut transcript = transcript_res?;
    if !transcript.words.is_empty() {
        diarize_words_local(audio_path, &mut transcript.words, seg_model, emb_model).await.ok();
    }
    Ok(transcript)
}

#[cfg(not(feature = "whisper"))]
pub async fn transcribe_with_whisper(
    _audio_path: &Path,
    _model_path: &Path,
    _seg_model: &Path,
    _emb_model: &Path,
) -> Result<TimestampedTranscript> {
    anyhow::bail!("Whisper not compiled in this build (feature disabled for CI). Use Deepgram or Vosk.")
}

/// Unified transcription entry point supporting Deepgram, Vosk, and Whisper
pub async fn transcribe_audio(
    audio_path: &Path,
    provider: &str,
    deepgram_api_key: &str,
    vosk_model_path: &Path,
    whisper_model_path: &Path,
    seg_model: &Path,
    emb_model: &Path,
) -> Result<TimestampedTranscript> {
    match provider.to_lowercase().as_str() {
        "vosk" => transcribe_with_vosk(audio_path, vosk_model_path, seg_model, emb_model).await,
        "whisper" => transcribe_with_whisper(audio_path, whisper_model_path, seg_model, emb_model).await,
        _ => transcribe_with_deepgram(audio_path, deepgram_api_key).await,
    }
}

#[cfg(feature = "diarize")]
pub async fn diarize_words_local(
    audio_path: &Path,
    words: &mut Vec<DeepgramWord>,
    _segmentation_model: &Path,
    _embedding_model: &Path,
) -> Result<()> {
    if words.is_empty() {
        return Ok(());
    }

    let audio_path_buf = audio_path.to_path_buf();
    let mut words_clone = words.clone();

    let res = tokio::task::spawn_blocking(move || -> Result<Vec<DeepgramWord>> {
        use speakrs::{ExecutionMode, OwnedDiarizationPipeline};

        let temp_wav = audio_path_buf.with_extension("pyannote_tmp.wav");
        let status = std::process::Command::new("ffmpeg")
            .args([
                "-y", "-i", audio_path_buf.to_str().unwrap_or_default(),
                "-vn", "-ac", "1", "-ar", "16000", "-f", "wav",
                temp_wav.to_str().unwrap_or_default(),
            ])
            .status()?;
        
        if !status.success() {
            anyhow::bail!("FFmpeg conversion failed");
        }

        let mut reader = hound::WavReader::open(&temp_wav)?;
        let samples: Vec<f32> = reader.samples::<i16>()
            .map(|s| (s.unwrap_or(0) as f32) / 32768.0)
            .collect();
        let _ = std::fs::remove_file(&temp_wav);

        let mut pipeline = OwnedDiarizationPipeline::from_pretrained(ExecutionMode::Cpu)?;
        let result = pipeline.run(&samples)?;

        struct SpeakerSegment { start: f64, end: f64, speaker: String }
        let segments: Vec<SpeakerSegment> = result.discrete_diarization.to_segments()
            .into_iter()
            .map(|s| SpeakerSegment {
                start: s.start as f64,
                end: s.end as f64,
                speaker: s.speaker,
            })
            .collect();

        println!("--> speakrs extracted {} speaker segments", segments.len());

        let mut speaker_map = std::collections::HashMap::new();
        let mut next_id = 0;

        for word in words_clone.iter_mut() {
            let word_mid = word.start + (word.end - word.start) / 2.0;
            let mut found = false;
            for segment in &segments {
                if word_mid >= segment.start && word_mid <= segment.end {
                    let spk_id = *speaker_map.entry(segment.speaker.clone()).or_insert_with(|| {
                        let id = next_id;
                        next_id += 1;
                        id
                    });
                    word.speaker = Some(spk_id);
                    found = true;
                    break;
                }
            }
            if !found {
                word.speaker = Some(0);
            }
        }
        
        Ok(words_clone)
    }).await?;

    if let Ok(updated_words) = res {
        *words = updated_words;
        tracing::info!("speakrs diarization applied successfully to {} words", words.len());
    }

    Ok(())
}

#[cfg(not(feature = "diarize"))]
pub async fn diarize_words_local(
    _audio_path: &Path,
    _words: &mut Vec<DeepgramWord>,
    _segmentation_model: &Path,
    _embedding_model: &Path,
) -> Result<()> {
    // Diarization disabled for this build (CI lite) — Deepgram already provides speaker tags
    Ok(())
}

/// Transcribe audio with Deepgram Nova-3
pub async fn transcribe_with_deepgram(
    audio_path: &Path,
    api_key: &str,
) -> Result<TimestampedTranscript> {
    info!("Transcribing with Deepgram Nova-3: {}", audio_path.display());

    let audio_bytes = tokio::fs::read(audio_path).await
        .context("Failed to read audio file")?;
    let file_size = audio_bytes.len();
    info!("Audio file size: {} bytes ({:.1} MB)", file_size, file_size as f64 / 1_048_576.0);

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.deepgram.com/v1/listen?model=nova-3&smart_format=true&punctuate=true&utterances=false&words=true&diarize_model=latest&language=en")
        .header("Authorization", format!("Token {}", api_key))
        .header("Content-Type", "audio/mpeg")
        .body(audio_bytes)
        .timeout(std::time::Duration::from_secs(600))
        .send()
        .await
        .context("Deepgram request failed")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Deepgram error {}: {}", status, body);
    }

    let json: serde_json::Value = response.json().await
        .context("Failed to parse Deepgram response")?;

    let alternative = json
        .pointer("/results/channels/0/alternatives/0")
        .context("Unexpected Deepgram response shape")?;

    let full_text = alternative["transcript"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let words: Vec<DeepgramWord> = alternative["words"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|w| {
            Some(DeepgramWord {
                word: w["word"].as_str()?.to_string(),
                start: w["start"].as_f64()?,
                end: w["end"].as_f64()?,
                confidence: w["confidence"].as_f64().unwrap_or(1.0),
                punctuated_word: w["punctuated_word"].as_str().map(|s| s.to_string()),
                speaker: w["speaker"].as_i64().map(|s| s as i32),
            })
        }).collect())
        .unwrap_or_default();

    let duration = words.last().map(|w| w.end).unwrap_or(0.0);
    let speakers: Vec<i32> = words.iter().filter_map(|w| w.speaker).collect();
    info!("Transcription complete: {} words, {:.1}s (diarized speakers: {:?})", words.len(), duration, speakers);

    Ok(TimestampedTranscript {
        full_text,
        words,
        duration,
    })
}

/// Split transcript into 30-minute chunks with 2-minute overlap
pub fn chunk_transcript(transcript: &TimestampedTranscript, chunk_mins: f64, overlap_mins: f64) -> Vec<Vec<&DeepgramWord>> {
    let chunk_secs = chunk_mins * 60.0;
    let overlap_secs = overlap_mins * 60.0;
    let mut chunks = Vec::new();
    let mut start = 0.0f64;

    loop {
        let end = start + chunk_secs;
        let chunk: Vec<&DeepgramWord> = transcript.words.iter()
            .filter(|w| w.start >= start && w.start < end)
            .collect();
        if chunk.is_empty() {
            break;
        }
        chunks.push(chunk);
        let actual_end = end;
        if actual_end >= transcript.duration {
            break;
        }
        start = (actual_end - overlap_secs).max(0.0);
    }
    chunks
}

/// Merge word-level diarization into `{start, end, speaker}` segments inside a
/// clip window, with timestamps shifted so the window starts at t=0.
pub fn speaker_segments_for_window(
    words: &[DeepgramWord],
    start: f64,
    end: f64,
) -> Vec<SpeakerSegment> {
    let mut segments: Vec<SpeakerSegment> = Vec::new();
    for w in words {
        if w.end < start || w.start > end {
            continue;
        }
        let speaker = match w.speaker {
            Some(s) => s,
            None => continue,
        };
        let s = (w.start - start).max(0.0);
        let e = (w.end - start).max(0.0);
        if let Some(last) = segments.last_mut() {
            if last.speaker == speaker && s <= last.end + 0.3 {
                last.end = e.max(last.end);
                continue;
            }
        }
        segments.push(SpeakerSegment {
            start: s,
            end: e,
            speaker,
        });
    }
    segments
}
