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
        generate_campaign_copy(cfg, &payload, &transcript, &source_title, &written_platforms).await?
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
        let row: Option<(String, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT file_path, transcript_text, hook_title FROM generated_clips WHERE task_id = ? ORDER BY clip_order ASC LIMIT 1"
        ).bind(source_task_id).fetch_optional(db).await?;
        if let Some((path, transcript, title)) = row {
            return Ok((PathBuf::from(path), transcript.unwrap_or_default(), title.unwrap_or_else(|| "Completed NovaClip task".into())));
        }
        anyhow::bail!("Selected source task has no generated video");
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
    let filter = format!("scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2,setsar=1");
    let status = Command::new("ffmpeg").args([
        "-y", "-i", source.to_str().unwrap_or_default(), "-t", &format!("{duration:.3}"),
        "-vf", &filter, "-c:v", "libx264", "-preset", "fast", "-crf", "20",
        "-c:a", "aac", "-b:a", "160k", "-movflags", "+faststart", output.to_str().unwrap_or_default(),
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
    let prompt = format!(r#"Create a platform-specific content campaign as strict JSON only.
Campaign: {}
Source: {}
Audience: {}
Goal: {}
Tone: {}
Core message: {}
CTA: {}
Instructions: {}
Platforms: {}

Write substantial, publish-ready content rather than short summaries. Preserve the source's facts and make each platform version meaningfully different.

For every selected platform return:
- 5 distinct hooks, each one or two complete sentences.
- A primary CTA and 2 CTA alternatives.
- 8-15 relevant hashtags or tags where the platform supports them.
- A short strategy note explaining the recommended angle, format, and posting approach.

Platform requirements:
- TikTok: caption of 120-220 words, 5 hooks, hashtags, CTA, on-screen text ideas, and a pinned-comment suggestion.
- Instagram: reel_caption of 180-300 words, short_caption of 60-100 words, 10-15 hashtags, story_sequence with 3-5 slides, carousel_outline with 5-8 slides, and CTA.
- YouTube: 8 title options, description of 300-500 words, short_description, 10-15 tags, 5 thumbnail_text options, chapter suggestions, and pinned_comment.
- LinkedIn: professional_post of 350-600 words, short_post of 120-200 words, 5 opening hooks, CTA, and 3 discussion questions.
- X: 5 standalone posts, a thread of 6-10 complete posts, 5 quote options, hashtags, and CTA.
- Newsletter: 8 subject_lines, 5 preview_text options, body of 600-1000 words with section headings, CTA section, and a short version of 250-400 words.
- Blog: 8 headlines, SEO description, detailed outline with 6-10 sections, introduction of 250-400 words, key talking points for every section, conclusion, and CTA.

Return every platform under its lowercase platform id. Return strict JSON only and do not omit requested fields.

Transcript:
{}"#,
        payload.campaign_name, source_title, payload.audience, payload.goal, payload.tone,
        payload.core_message, payload.cta, payload.instructions, ids.join(", "), transcript);
    call_json_model(cfg, &prompt).await
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
