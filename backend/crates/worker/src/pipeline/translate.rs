use anyhow::{Context, Result};
use serde_json::{json, Value};
use tracing::info;

const TRANSLATE_SYSTEM_PROMPT: &str = r##"You are a caption translator. Given a list of words with timestamps, translate them to the target language. Keep the same number of words and same timestamps. Return a JSON array of objects with "word", "start", and "end" fields.
Input format: [{"word": "hello", "start": 0.0, "end": 0.5}, ...]
Output format: same structure with translated words."##;

/// Translate a list of caption word references to the target language using Gemini.
/// Keeps timestamps unchanged, replaces word text with translated text.
pub async fn translate_words(
    words: &[&crate::pipeline::transcribe::DeepgramWord],
    target_language: &str,
    model: &str,
    api_key: &str,
) -> Result<Vec<crate::pipeline::transcribe::DeepgramWord>> {
    if target_language.is_empty() || words.is_empty() {
        return Ok(words.iter().map(|w| (*w).clone()).collect());
    }

    let words_json: Vec<Value> = words.iter().map(|w| {
        json!({"word": w.word, "start": w.start, "end": w.end})
    }).collect();

    let body = json!({
        "systemInstruction": {
            "parts": [{"text": TRANSLATE_SYSTEM_PROMPT}]
        },
        "contents": [{
            "role": "user",
            "parts": [{"text": format!("Translate to {}:\n{}", target_language, serde_json::to_string(&words_json).unwrap())}]
        }],
        "generationConfig": {
            "temperature": 0.1,
            "maxOutputTokens": 8192,
            "responseMimeType": "application/json"
        }
    });

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await
        .context("Gemini translate request failed")?;

    if !response.status().is_success() {
        anyhow::bail!("Gemini translate error: {}", response.status());
    }

    let resp: Value = response.json().await.context("Failed to parse Gemini translate response")?;
    let content_text = resp
        .pointer("/candidates/0/content/parts/0/text")
        .and_then(|v| v.as_str())
        .context("No text in Gemini translate response")?;

    let clean = content_text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let translated: Vec<Value> = serde_json::from_str(clean)
        .context("Failed to parse translated words JSON")?;

    let result: Vec<crate::pipeline::transcribe::DeepgramWord> = translated.iter()
        .filter_map(|v| {
            Some(crate::pipeline::transcribe::DeepgramWord {
                word: v["word"].as_str()?.to_string(),
                start: v["start"].as_f64().unwrap_or(0.0),
                end: v["end"].as_f64().unwrap_or(0.0),
                confidence: 1.0,
                punctuated_word: Some(v["word"].as_str()?.to_string()),
            })
        })
        .collect();

    info!("Translated {} words to {}", result.len(), target_language);
    Ok(result)
}
