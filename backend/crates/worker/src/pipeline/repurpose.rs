use anyhow::{Context, Result};
use novaclip_db::DbPool;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::info;

use crate::{emit_progress, PipelineConfig};
use crate::pipeline::crop::output_dimensions;
use crate::pipeline::download::{download_youtube, extract_audio, get_video_duration, resolve_upload_path};
use crate::pipeline::transcribe::{build_transcript_for_prompt, transcribe_audio};

#[derive(Debug, Clone, Deserialize)]
struct RepurposePayload {
    campaign_name: String,
    #[serde(default)] audience: String,
    #[serde(default)] goal: String,
    #[serde(default)] tone: String,
    #[serde(default)] core_message: String,
    #[serde(default)] cta: String,
    #[serde(default)] instructions: String,
    #[serde(default)] source_task_id: Option<String>,
    #[serde(default)] source_url: Option<String>,
    #[serde(default)] platforms: Vec<PlatformRequest>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct PlatformRequest {
    id: String,
    #[serde(default)] video: bool,
    #[serde(default)] written: bool,
    #[serde(default = "default_ratio")] aspect_ratio: String,
    #[serde(default)] duration_seconds: Option<i32>,
}

fn default_ratio() -> String { "9:16".into() }

pub async fn process_repurpose_task(db: &DbPool, cfg: &PipelineConfig, output_dir: &Path, task_id: &str) -> Result<()> {
    let payload: RepurposePayload = serde_json::from_value(
        cfg.repurpose_payload.clone().context("Missing Nova Repurpose payload")?
    ).context("Invalid Nova Repurpose payload")?;

    emit_progress(db, task_id, 8, "Resolving source content...", "processing").await;
    let (source_video, transcript, source_title) = resolve_source(db, cfg, &payload).await?;

    emit_progress(db, task_id, 28, "Building campaign strategy...", "processing").await;
    let written_platforms: Vec<&PlatformRequest> = payload.platforms.iter().filter(|p| p.written).collect();
    let copy = if written_platforms.is_empty() {
        json!({})
    } else {
        normalize_copy(generate_campaign_copy(cfg, &payload, &transcript, &source_title, &written_platforms).await?)
    };

    emit_progress(db, task_id, 50, "Creating platform video versions...", "processing").await;
    let video_platforms: Vec<&PlatformRequest> = payload.platforms.iter().filter(|p| p.video).collect();
    let source_duration = get_video_duration(&source_video).await.unwrap_or(0.0);
    let mut videos = Vec::new();
    for (order, platform) in video_platforms.iter().enumerate() {
        let filename = format!("{}_video.mp4", platform.id);
        let output = output_dir.join(&filename);
        let duration = platform.duration_seconds.map(|d| d as f64).unwrap_or(source_duration).min(source_duration).max(0.1);
        render_variant(&source_video, &output, &platform.aspect_ratio, duration).await?;
        insert_clip(db, task_id, order as i32, &filename, &output, duration, &platform.id).await?;
        videos.push(json!({
            "platform": platform.id,
            "filename": filename,
            "aspect_ratio": platform.aspect_ratio,
            "duration": duration
        }));
        let pct = 50 + (((order + 1) * 30) / video_platforms.len().max(1)) as i32;
        emit_progress(db, task_id, pct, &format!("Created {} video...", platform.id), "processing").await;
    }

    emit_progress(db, task_id, 88, "Finalizing campaign package...", "processing").await;
    let result = json!({
        "campaign_name": payload.campaign_name,
        "source_title": source_title,
        "audience": payload.audience,
        "goal": payload.goal,
        "tone": payload.tone,
        "core_message": payload.core_message,
        "cta": payload.cta,
        "platform_copy": copy,
        "videos": videos,
        "platforms": payload.platforms,
    });
    sqlx::query("UPDATE tasks SET repurpose_result = ?, status = 'completed', progress = 100, progress_message = 'Repurpose campaign ready!', completed_at = datetime('now'), updated_at = datetime('now') WHERE id = ?")
        .bind(serde_json::to_string(&result)?)
        .bind(task_id)
        .execute(db).await?;
    info!("Nova Repurpose {} completed", task_id);
    Ok(())
}

async fn resolve_source(db: &DbPool, cfg: &PipelineConfig, payload: &RepurposePayload) -> Result<(PathBuf, String, String)> {
    if let Some(source_task_id) = payload.source_task_id.as_deref() {
        // Use the completed task's ORIGINAL source video and full transcript,
        // not its generated clips.
        let row: Option<(String, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT source_url, source_title, transcript_cache FROM tasks WHERE id = ?"
        ).bind(source_task_id).fetch_optional(db).await?;
        let (source_url, title, transcript_cache) = row
            .ok_or_else(|| anyhow::anyhow!("Selected source task not found"))?;

        let video = if source_url.starts_with("upload://") {
            resolve_upload_path(&source_url, &cfg.temp_dir)
        } else {
            download_youtube(&source_url, &cfg.temp_dir).await?
        };
        if !video.exists() {
            anyhow::bail!("Original source video not found on disk: {}", video.display());
        }

        // Prefer the cached full-source transcript to avoid re-transcribing.
        let transcript = match transcript_cache {
            Some(t) if !t.trim().is_empty() => t,
            _ => {
                let audio = extract_audio(&video, &cfg.temp_dir).await?;
                let tr = transcribe_audio(
                    &audio, &cfg.stt_provider, &cfg.deepgram_api_key,
                    Path::new(&cfg.vosk_model_path), Path::new(&cfg.whisper_model_path),
                    Path::new(&cfg.pyannote_segmentation_model_path), Path::new(&cfg.pyannote_embedding_model_path),
                ).await?;
                let _ = tokio::fs::remove_file(audio).await;
                build_transcript_for_prompt(&tr)
            }
        };
        let title = title.filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| "Completed NovaClip source".into());
        return Ok((video, transcript, title));
    }

    let source_url = payload.source_url.as_deref().unwrap_or(&cfg.url);
    let video = if source_url.starts_with("upload://") {
        resolve_upload_path(source_url, &cfg.temp_dir)
    } else {
        download_youtube(source_url, &cfg.temp_dir).await?
    };
    let audio = extract_audio(&video, &cfg.temp_dir).await?;
    let transcript = transcribe_audio(
        &audio, &cfg.stt_provider, &cfg.deepgram_api_key,
        Path::new(&cfg.vosk_model_path), Path::new(&cfg.whisper_model_path),
        Path::new(&cfg.pyannote_segmentation_model_path), Path::new(&cfg.pyannote_embedding_model_path),
    ).await?;
    let _ = tokio::fs::remove_file(audio).await;
    Ok((video, build_transcript_for_prompt(&transcript), payload.campaign_name.clone()))
}

async fn render_variant(source: &Path, output: &Path, aspect_ratio: &str, duration: f64) -> Result<()> {
    let (w, h) = output_dimensions(aspect_ratio);
    // Fill the target frame: scale source to fully cover as a blurred background,
    // then overlay the fit-to-frame source centered on top. This avoids letterbox
    // black bars and makes vertical sources read correctly in 16:9 / 1:1 frames.
    let filter = format!(
        "[0:v]split=2[bg][fg];\
         [bg]scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h},boxblur=20:4,setsar=1[bgb];\
         [fg]scale={w}:{h}:force_original_aspect_ratio=decrease,setsar=1[fgf];\
         [bgb][fgf]overlay=(W-w)/2:(H-h)/2,setsar=1,format=yuv420p[vout]"
    );
    let status = Command::new("ffmpeg").args([
        "-y", "-i", source.to_str().unwrap_or_default(), "-t", &format!("{duration:.3}"),
        "-filter_complex", &filter, "-map", "[vout]", "-map", "0:a:0?",
        "-c:v", "libx264", "-preset", "fast", "-crf", "20",
        "-c:a", "aac", "-b:a", "160k", "-movflags", "+faststart", "-shortest",
        output.to_str().unwrap_or_default(),
    ]).status().await.context("Failed to render repurpose variant")?;
    if !status.success() { anyhow::bail!("FFmpeg failed to create repurpose variant"); }
    Ok(())
}

async fn insert_clip(db: &DbPool, task_id: &str, order: i32, filename: &str, path: &Path, duration: f64, platform: &str) -> Result<()> {
    sqlx::query(r#"INSERT INTO generated_clips
        (id, task_id, clip_order, filename, file_path, start_time, end_time, duration,
         transcript_text, relevance_score, virality_score, hook_score, engagement_score,
         value_score, shareability_score, hook_type, hook_title, reasoning)
        VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"#)
        .bind(uuid::Uuid::new_v4().to_string()).bind(task_id).bind(order).bind(filename)
        .bind(path.to_str().unwrap_or_default()).bind("0:00").bind(format!("{:.1}s", duration)).bind(duration)
        .bind(Option::<String>::None).bind(1.0).bind(90).bind(90).bind(90).bind(90).bind(90)
        .bind(Some("repurpose".to_string())).bind(Some(format!("{} platform version", platform)))
        .bind(Some(format!("Adapted for {}", platform))).execute(db).await?;
    Ok(())
}

async fn generate_campaign_copy(cfg: &PipelineConfig, payload: &RepurposePayload, transcript: &str, source_title: &str, platforms: &[&PlatformRequest]) -> Result<Value> {
    let ids: Vec<&str> = platforms.iter().map(|p| p.id.as_str()).collect();
    let prompt = format!(r#"You are a senior professional content strategist and copywriter. Produce complete, publish-ready, human-sounding content for each selected platform, varying in length and structure to meet each platform's industry-standard best practices.

Campaign: {}
Source: {}
Audience: {}
Goal: {}
Tone: {}
Core message: {}
CTA: {}
Instructions: {}
Platforms: {}

Return a strict JSON object whose top-level keys are EXACTLY the lowercase platform ids: {}. Do NOT wrap them under "platforms" or any other key, and do not include campaign/audience/goal/tone/source metadata keys in the output object.

WRITING STANDARDS (apply to every platform):
- Write like a talented human marketer, never like AI. Use varied sentence length, concrete detail, and a natural rhythm.
- Follow standard professional grammar, spelling, and punctuation.
- Content must be sufficiently long and complete. Do not summarize or truncate; deliver the full publish-ready piece.
- Give each platform meaningfully different content and angles derived from the source facts below.
- For every selected platform also include a "strategy_note" field (one paragraph) explaining the best posting approach.

FORMATTING (strict):
- Use ONLY plain ASCII characters. No emojis, no em-dashes (—), no smart/curly quotes, no ellipsis characters, no exotic unicode symbols. Use straight quotes and regular hyphens.
- Arrays are plain lists of strings. Never use Markdown code fences or JSON wrappers in output.

PLATFORM SPECIFICS (follow these industry-standard lengths and structures):
- tiktok: "caption" (120-220 words), "hooks" (5), "hashtags" (8-15), "cta", "onscreen_text" (3), "pinned_comment", "strategy_note".
- instagram: "reel_caption" (180-300 words), "short_caption" (60-100 words), "hooks" (5), "hashtags" (10-15), "story_sequence" (3-5 short lines), "carousel_outline" (5-8 headings), "cta", "strategy_note".
- youtube: "titles" (8), "description" (300-500 words), "short_description" (1-2 sentences), "tags" (10-15), "thumbnail_text" (5), "chapters" (list of timestamp headings), "pinned_comment", "cta", "strategy_note".
- linkedin: "post" (350-600 words), "short_post" (120-200 words), "hooks" (5), "discussion_questions" (3), "cta", "hashtags" (5), "strategy_note".
- x: "tweets" (5 standalone posts), "thread" (6-10 complete posts), "hashtags", "cta", "strategy_note".
- newsletter: "subject_lines" (8), "preview_text" (5), "body" (600-1000 words with section headings), "short_version" (250-400 words), "cta", "strategy_note".
- blog: "headlines" (8), "seo_description", "introduction" (250-400 words), "outline" (6-10 section headings), "talking_points" (map of each section to key bullet points), "conclusion" (200-300 words), "cta", "strategy_note".

Return strict JSON only. Provide every platform and every listed field; omit nothing.

Transcript:
{}"#,
        payload.campaign_name, source_title, payload.audience, payload.goal, payload.tone,
        payload.core_message, payload.cta, payload.instructions, ids.join(", "), ids.join(", "), transcript);
    call_json_model(cfg, &prompt).await
}

/// Normalize the model output so `platform_copy` is a flat map of platform_id -> content,
/// stripping any metadata wrapper the model may have returned, and sanitizing text.
fn normalize_copy(value: Value) -> Value {
    let mut obj = match value {
        Value::Object(map) => map,
        _ => return value,
    };
    if let Some(inner) = obj.remove("platforms").and_then(|v| v.as_object().cloned()) {
        obj = inner;
    }
    for meta in ["audience", "campaign", "goal", "tone", "source", "core_message", "cta"] {
        obj.remove(meta);
    }
    Value::Object(obj.into_iter().map(|(k, v)| (k, sanitize_value(v))).collect())
}

/// Recursively strip non-ASCII characters from strings so no emojis, smart quotes,
/// em-dashes, or other exotic unicode leak into the generated copy.
fn sanitize_value(value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.into_iter().map(sanitize_value).collect()),
        Value::Object(map) => Value::Object(map.into_iter().map(|(k, v)| (k, sanitize_value(v))).collect()),
        Value::String(s) => Value::String(sanitize_text(&s)),
        other => other,
    }
}

fn sanitize_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii() || c.is_whitespace() {
            out.push(c);
        }
    }
    out.trim().to_string()
}

async fn call_json_model(cfg: &PipelineConfig, prompt: &str) -> Result<Value> {
    if cfg.llm_provider.contains('/') {
        let response = reqwest::Client::new().post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", cfg.openrouter_api_key))
            .header("HTTP-Referer", "https://novaclip.app").header("X-Title", "NovaClip")
            .json(&json!({"model": cfg.llm_provider, "messages":[{"role":"system","content":"You are a senior social content strategist and long-form copywriter. Produce complete, publish-ready work and return strict JSON only."},{"role":"user","content":prompt}], "temperature":0.55, "max_tokens":12000}))
            .send().await?;
        if !response.status().is_success() { anyhow::bail!("OpenRouter campaign generation failed: {}", response.status()); }
        let body: Value = response.json().await?;
        let text = body.pointer("/choices/0/message/content").and_then(Value::as_str).unwrap_or("{}");
        return parse_json(text);
    }
    let response = reqwest::Client::new().post(format!("https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}", cfg.llm_provider, cfg.gemini_api_key))
        .json(&json!({"contents":[{"role":"user","parts":[{"text":prompt}]}],"generationConfig":{"responseMimeType":"application/json","maxOutputTokens":12000,"temperature":0.55}}))
        .send().await?;
    if !response.status().is_success() { anyhow::bail!("Gemini campaign generation failed: {}", response.status()); }
    let body: Value = response.json().await?;
    let text = body.pointer("/candidates/0/content/parts/0/text").and_then(Value::as_str).unwrap_or("{}");
    parse_json(text)
}

fn parse_json(text: &str) -> Result<Value> {
    let clean = text.trim().trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim();
    serde_json::from_str(clean).context("Failed to parse campaign JSON")
}
