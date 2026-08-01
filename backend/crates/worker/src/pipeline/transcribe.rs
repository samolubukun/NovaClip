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
