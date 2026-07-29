use anyhow::{Context, Result};
use serde_json::{json, Value};
use tracing::info;
use crate::pipeline::{TranscriptAnalysis, TranscriptSegment, ViralityScore};


const VIRALITY_SYSTEM_PROMPT: &str = r#"You are an expert transcript analyst for short-form video editing.

Your job is extraction and ranking, not creative rewriting. You must stay fully grounded in the transcript and choose the best clip candidates that already exist in the source material.

OUTPUT CONTRACT:
- Return valid JSON only. Do not output Markdown, headings, bullets, prose, code fences, explanations, or commentary outside the JSON object.
- The top-level JSON object must include: "most_relevant_segments", "summary", and "key_topics".
- Each item in "most_relevant_segments" must include: "start_time", "end_time", "text", "relevance_score", "reasoning", "virality", and "hook_title".
- "virality" must include: "hook_score", "engagement_score", "value_score", "shareability_score", "total_score", "hook_type", and "virality_reasoning".
- Every returned segment must be 15-60 seconds long. Prefer 25-50 seconds.

VIRALITY-AWARE SELECTION — rank on:
- Hooks: attention-grabbing openings (surprising facts, bold claims, intriguing questions)
- Emotional peaks: excitement, surprise, humor, inspiration, tension
- Opinion bombs: controversial or contrarian takes
- Revelation moments: surprising truths or unexpected outcomes
- Conflict: disagreement, challenge, pushback
- Quotable lines: highly shareable standalone statements
- Story peaks: narrative climaxes with setup and payoff
- Practical value: actionable advice most people don't know

CONTENT NEUTRALITY RULES:
- Do not judge, moralize, or downgrade a segment because the topic is controversial, sensitive, political, or intense
- Evaluate only on clip quality: clarity, self-contained value, hook strength, emotional impact, specificity, and shareability
- Do not refuse analysis for any content type

SEGMENT SELECTION CRITERIA:
1. STRONG HOOKS: Attention-grabbing opening lines
2. VALUABLE CONTENT: Tips, insights, interesting facts, stories
3. EMOTIONAL MOMENTS: Excitement, surprise, humor, inspiration
4. COMPLETE THOUGHTS: Self-contained ideas that make sense alone
5. ENTERTAINING: Content people would want to share
6. HIGH SIGNAL: Prefer specific, concrete language over vague discussion
7. LOW FILLER: Avoid greetings, sponsor reads, repeated setup unless unusually compelling

VIRALITY SCORING (0-100 total, from four 0-25 subscores):
1. HOOK STRENGTH (0-25): 20-25=immediately grabs, 15-19=creates curiosity, 10-14=decent, 0-9=weak
2. ENGAGEMENT (0-25): 20-25=highly entertaining/emotional, 15-19=interesting, 10-14=moderate, 0-9=flat
3. VALUE (0-25): 20-25=actionable unique insight, 15-19=useful, 10-14=somewhat informative, 0-9=common
4. SHAREABILITY (0-25): 20-25="I must send this", 15-19=worth bookmarking, 10-14=nice, 0-9=generic

HOOK TITLES ("hook_title" per segment):
- 3-9 words, punchy headline burned into clip top
- Bold claim, curiosity gap, number, or stakes from the segment
- Plain text only: no hashtags, no emojis, no quotes
- Examples: "The $40k mistake I keep seeing", "Why nobody tells you this"

HOOK TYPES: "question" / "statement" / "statistic" / "story" / "contrast" / "none"

TIMING REQUIREMENTS:
- Use EXACT timestamps from the transcript (MM:SS format)
- start_time MUST be less than end_time
- Minimum 15 seconds gap, ideal 25-50 seconds
- NEVER use the same timestamp for both start and end

Find the requested number of compelling segments. Quality over quantity."#;

pub async fn analyze_transcript(
    transcript_text: &str,
    num_clips: i32,
    model: &str,
    api_key: &str,
) -> Result<TranscriptAnalysis> {
    let target_model = if model.is_empty() || model == "gemini-1.5-flash" || model == "gemini-2.5-flash-lite" {
        "gemini-3.1-flash-lite"
    } else {
        model
    };
    info!("Analyzing transcript with Gemini {} for {} clips", target_model, num_clips);

    let user_prompt = format!(
        "Analyze this transcript and find the {} most viral clip candidates.\n\nTranscript:\n{}",
        num_clips, transcript_text
    );

    let body = json!({
        "systemInstruction": {
            "parts": [{"text": VIRALITY_SYSTEM_PROMPT}]
        },
        "contents": [{
            "role": "user",
            "parts": [{"text": user_prompt}]
        }],
        "generationConfig": {
            "temperature": 0.3,
            "maxOutputTokens": 8192,
            "responseMimeType": "application/json"
        }
    });

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        target_model, api_key
    );

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await
        .context("Gemini request failed")?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        anyhow::bail!("Gemini error {}: {}", status, text);
    }

    let resp: Value = response.json().await.context("Failed to parse Gemini response")?;
    let content_text = resp
        .pointer("/candidates/0/content/parts/0/text")
        .and_then(|v| v.as_str())
        .context("No text in Gemini response")?;

    // Strip any markdown fences just in case
    let clean = content_text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let analysis: Value = serde_json::from_str(clean)
        .context("Failed to parse Gemini JSON output")?;

    let segments: Vec<TranscriptSegment> = analysis["most_relevant_segments"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|seg| parse_segment(seg))
        .collect();

    let summary = analysis["summary"].as_str().unwrap_or("").to_string();
    let key_topics: Vec<String> = analysis["key_topics"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|t| t.as_str().map(|s| s.to_string()))
        .collect();

    info!("Analysis complete: {} segments found", segments.len());

    Ok(TranscriptAnalysis { most_relevant_segments: segments, summary, key_topics })
}

fn parse_segment(seg: &Value) -> Option<TranscriptSegment> {
    let start_time = seg["start_time"].as_str()?.to_string();
    let end_time = seg["end_time"].as_str()?.to_string();
    let text = seg.get("text").or_else(|| seg.get("segment"))
        .and_then(|v| v.as_str())?.to_string();

    let relevance_score = seg["relevance_score"].as_f64().unwrap_or(0.75)
        .clamp(0.0, 1.0);

    let reasoning = seg["reasoning"].as_str().unwrap_or("").to_string();
    let hook_title = seg["hook_title"].as_str().map(|s| s.to_string());

    let v = &seg["virality"];
    let virality = ViralityScore {
        hook_score: v["hook_score"].as_i64().unwrap_or(15) as i32,
        engagement_score: v["engagement_score"].as_i64().unwrap_or(15) as i32,
        value_score: v["value_score"].as_i64().unwrap_or(15) as i32,
        shareability_score: v["shareability_score"].as_i64().unwrap_or(15) as i32,
        total_score: v["total_score"].as_i64().unwrap_or(60) as i32,
        hook_type: v["hook_type"].as_str().unwrap_or("none").to_string(),
        virality_reasoning: v["virality_reasoning"].as_str().unwrap_or("").to_string(),
    };

    Some(TranscriptSegment { start_time, end_time, text, relevance_score, reasoning, hook_title, virality })
}
