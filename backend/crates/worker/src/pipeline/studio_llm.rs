use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptItem {
    pub sentence: String,
    pub keyword: String,
}

pub struct StudioLlmProcessor {
    pub api_key: String,
    pub provider_or_model: String,
    pub openrouter_key: String,
}

impl StudioLlmProcessor {
    pub fn new(api_key: String, provider_or_model: String, openrouter_key: String) -> Self {
        Self { api_key, provider_or_model, openrouter_key }
    }

    /// Decomposes a video script into sentence-level items with visual search keywords
    pub async fn decompose_script(&self, script: &str, target_duration_secs: i32) -> Result<Vec<ScriptItem>> {
        info!("Decomposing script for visual keywords using {} (target {}s)", self.provider_or_model, target_duration_secs);

        let max_items = (target_duration_secs as f64 / 6.0).ceil() as i32;
        let system_prompt = format!(r#"You are an AI video producer. Break down the user's script into individual sentences or natural thought chunks.
For each sentence, provide a specific visual stock video search keyword (1-3 words) to search on stock footage sites (Pexels, Pixabay, Pinterest).

Target: ~{max_items} scenes for a {target_duration_secs}-second video. Each scene = one sentence + one keyword.
Aim for {target_items} items total (fewer for slow pace, more for fast-paced).

OUTPUT FORMAT:
Return a JSON array of objects only. Do NOT output markdown formatting or extra text.
Each object must have:
- "sentence": "The exact sentence text"
- "keyword": "1-3 word visual search term"

Example:
[
  {{"sentence": "Space is vastly empty yet full of mystery.", "keyword": "galaxy stars"}},
  {{"sentence": "Black holes consume everything in their path.", "keyword": "black hole space"}}
]"#,
            max_items = max_items,
            target_duration_secs = target_duration_secs,
            target_items = max_items,
        );

        if self.provider_or_model.contains('/') || !self.openrouter_key.is_empty() {
            self.call_openrouter(&system_prompt, script).await
        } else {
            self.call_gemini(&system_prompt, script).await
        }
    }

    const GEMINI_DEFAULT_MODEL: &str = "gemini-3.1-flash-lite";

    /// Extracts error message from a Gemini API error response
    fn gemini_error(resp: &serde_json::Value) -> Option<String> {
        resp.pointer("/error/message").and_then(|v| v.as_str()).map(|s| s.to_string())
    }

    /// Generates a full faceless video script from a user topic prompt
    pub async fn generate_topic_script(&self, topic: &str, vibe: &str, duration_secs: i32) -> Result<String> {
        info!("Generating script for topic '{}' with vibe '{}' ({}s)", topic, vibe, duration_secs);

        // Estimate ~3 words per second for natural speech
        let target_words = (duration_secs as f64 * 2.8) as i32;
        let prompt = format!(
            "Write an engaging {duration_secs}-second faceless short video script on the topic: '{topic}'. Vibe: {vibe}. \
             Target ~{target_words} words total. Break into 5-{max_clips} short punchy sentences, each a separate scene. \
             Hook in first 3 seconds. Plain text script only.",
            duration_secs = duration_secs,
            topic = topic,
            vibe = vibe,
            target_words = target_words,
            max_clips = (duration_secs / 8).max(2),
        );

        if self.provider_or_model.contains('/') || !self.openrouter_key.is_empty() {
            let res = self.call_openrouter("You are a viral script writer.", &prompt).await?;
            Ok(res.iter().map(|item| item.sentence.clone()).collect::<Vec<_>>().join(" "))
        } else {
            let model = if self.provider_or_model.is_empty() { Self::GEMINI_DEFAULT_MODEL } else { &self.provider_or_model };
            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                model, self.api_key
            );

            let body = json!({
                "contents": [{"role": "user", "parts": [{"text": prompt}]}]
            });

            let client = Client::new();
            let response = client.post(&url).json(&body).send().await.context("Gemini script gen request failed")?;
            let resp: Value = response.json().await.context("Failed to parse Gemini response")?;
            if let Some(err) = Self::gemini_error(&resp) {
                anyhow::bail!("Gemini API error: {}", err);
            }
            let text = resp.pointer("/candidates/0/content/parts/0/text").and_then(|v| v.as_str()).unwrap_or_default();
            Ok(text.to_string())
        }
    }

    async fn call_gemini(&self, system_prompt: &str, user_text: &str) -> Result<Vec<ScriptItem>> {
        let model = if self.provider_or_model.is_empty() { Self::GEMINI_DEFAULT_MODEL } else { &self.provider_or_model };
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, self.api_key
        );

        let body = json!({
            "systemInstruction": {"parts": [{"text": system_prompt}]},
            "contents": [{"role": "user", "parts": [{"text": user_text}]}],
            "generationConfig": {"temperature": 0.2, "responseMimeType": "application/json"}
        });

        let client = Client::new();
        let response = client.post(&url).json(&body).send().await.context("Gemini decompose request failed")?;
        let resp: Value = response.json().await.context("Failed to parse Gemini response")?;
        if let Some(err) = Self::gemini_error(&resp) {
            anyhow::bail!("Gemini API error: {}", err);
        }
        let text = resp.pointer("/candidates/0/content/parts/0/text").and_then(|v| v.as_str()).unwrap_or("[]");
        let clean = text.trim().trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim();
        let items: Vec<ScriptItem> = serde_json::from_str(clean).unwrap_or_default();
        Ok(items)
    }

    async fn call_openrouter(&self, system_prompt: &str, user_text: &str) -> Result<Vec<ScriptItem>> {
        let model = if self.provider_or_model.is_empty() { "openrouter/free" } else { &self.provider_or_model };
        let url = "https://openrouter.ai/api/v1/chat/completions";

        let body = json!({
            "model": model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_text}
            ],
            "temperature": 0.2
        });

        let client = Client::new();
        let response = client.post(url)
            .header("Authorization", format!("Bearer {}", self.openrouter_key))
            .header("HTTP-Referer", "https://novaclip.app")
            .header("X-Title", "NovaClip")
            .json(&body)
            .send()
            .await
            .context("OpenRouter request failed")?;

        let resp: Value = response.json().await.context("Failed to parse OpenRouter response")?;
        let text = resp.pointer("/choices/0/message/content").and_then(|v| v.as_str()).unwrap_or("[]");
        let clean = text.trim().trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim();
        let items: Vec<ScriptItem> = serde_json::from_str(clean).unwrap_or_default();
        Ok(items)
    }
}
