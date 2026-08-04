use std::collections::HashMap;
use std::path::PathBuf;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Response, Sse},
    routing::{delete, get, patch, post},
    Json, Router,
};
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::convert::Infallible;
use std::time::Duration;
use uuid::Uuid;

use axum::response::sse::{Event, KeepAlive};
use crate::state::AppState;
use novaclip_db::{GeneratedClip, Task};

pub fn tasks_router() -> Router<AppState> {
    Router::new()
        .route("/tasks", get(list_tasks).post(create_task))
        .route("/studio/generate_script", post(generate_studio_script))
        .route("/tasks/{id}", get(get_task).delete(delete_task).patch(update_task))
        .route("/tasks/{id}/progress", get(task_progress_sse))
        .route("/tasks/{id}/download-all", get(download_all_clips))
        .route("/tasks/{id}/cancel", post(cancel_task))
        .route("/tasks/{id}/resume", post(resume_task))
        .route("/tasks/{id}/settings", post(apply_settings))
        .route("/tasks/{id}/clips/{clip_id}", delete(delete_clip).patch(trim_clip))
        .route("/tasks/{id}/clips/{clip_id}/split", post(split_clip))
        .route("/tasks/{id}/clips/{clip_id}/regenerate", post(regenerate_clip))
        .route("/tasks/{id}/clips/{clip_id}/captions", patch(update_captions))
        .route("/tasks/{id}/clips/merge", post(merge_clips))
        .route("/tasks/{id}/watermark", post(upload_watermark))
        .route("/tasks/ai-prompt", post(ai_prompt_handler))
        .route("/tasks/ai-prompt/chat", post(ai_chat_handler))
        .route("/tasks/{id}/approve-edit-plan", post(approve_edit_plan))
        .route("/tasks/{id}/replan", post(replan_task))
        .route("/tasks/{id}/repurpose-pdf", get(repurpose_pdf))
}

#[derive(Deserialize)]
struct ListQuery {
    limit: Option<i64>,
}

async fn list_tasks(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let limit = q.limit.unwrap_or(50);

    let rows = sqlx::query_as::<_, Task>(
        "SELECT * FROM tasks ORDER BY created_at DESC LIMIT ?"
    )
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;

    // Fetch clip counts in a single query to fix the always-zero bug
    let ids: Vec<String> = rows.iter().map(|t| format!("'{}'", t.id)).collect();
    let mut count_map: HashMap<String, i64> = HashMap::new();
    if !ids.is_empty() {
        let query = format!(
            "SELECT task_id, COUNT(*) as cnt FROM generated_clips WHERE task_id IN ({}) GROUP BY task_id",
            ids.join(",")
        );
        let counts: Vec<(String, i64)> = sqlx::query_as(&query)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();
        for (tid, cnt) in counts {
            count_map.insert(tid, cnt);
        }
    }

    let tasks: Vec<Value> = rows.into_iter().map(|t| {
        let clips_count = count_map.get(&t.id.to_string()).copied().unwrap_or(0);
        json!({
            "id": t.id,
            "status": t.status,
            "progress": t.progress,
            "source_url": t.source_url,
            "source_title": t.source_title,
            "source_type": t.source_type,
            "clips_count": clips_count,
            "created_at": t.created_at,
            "completed_at": t.completed_at,
        })
    }).collect();

    let total = tasks.len();
    Ok(Json(json!({"tasks": tasks, "total": total})))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioApiKeys {
    #[serde(default)]
    pub gemini_key: Option<String>,
    #[serde(default)]
    pub openrouter_key: Option<String>,
    #[serde(default)]
    pub deepgram_key: Option<String>,
    #[serde(default)]
    pub elevenlabs_key: Option<String>,
    #[serde(default)]
    pub pexels_key: Option<String>,
    #[serde(default)]
    pub pixabay_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioPayload {
    pub script: String,
    #[serde(default)]
    pub duration: Option<i32>,
    #[serde(default)]
    pub llm_provider: Option<String>,
    #[serde(default)]
    pub tts_provider: Option<String>,
    #[serde(default)]
    pub voice: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub media_type: Option<String>,
    #[serde(default)]
    pub vibe: Option<String>,
    #[serde(default)]
    pub subtitle_style: Option<String>,
    #[serde(default)]
    pub bg_music: Option<String>,
    #[serde(default)]
    pub api_keys: Option<StudioApiKeys>,
}

#[derive(Deserialize)]
struct CreateTaskRequest {
    url: Option<String>,
    source: Option<Value>,
    aspect_ratio: Option<String>,
    num_clips: Option<i32>,
    font_family: Option<String>,
    font_size: Option<i32>,
    font_color: Option<String>,
    caption_template: Option<String>,
    add_subtitles: Option<bool>,
    include_broll: Option<bool>,
    processing_mode: Option<String>,
    cut_long_pauses: Option<bool>,
    pause_threshold_ms: Option<i32>,
    remove_filler_words: Option<bool>,
    filtered_words: Option<Vec<String>>,
    gemini_api_key: Option<String>,
    llm_provider: Option<String>,
    deepgram_api_key: Option<String>,
    openrouter_api_key: Option<String>,
    stt_provider: Option<String>,
    auto_vertical_reframe: Option<bool>,
    reframe_preset: Option<String>,
    reframe_frame_skip: Option<i32>,
    reframe_layout: Option<String>,
    speaker_active_switch: Option<bool>,
    split_divider: Option<bool>,
    originality_boost: Option<String>,
    translate_language: Option<String>,
    giphy_api_key: Option<String>,
    source_title: Option<String>,
    /// Studio payload for faceless AI video generation tasks (source_type = "studio")
    studio_payload: Option<StudioPayload>,
    /// NovaEdit payload for agentic editing tasks (source_type = "agentic")
    novaedit_payload: Option<serde_json::Value>,
    repurpose_payload: Option<serde_json::Value>,
    highlight_color: Option<String>,
    caption_animation: Option<String>,
    auto_emojis: Option<bool>,
    watermark_position: Option<String>,
    watermark_opacity: Option<f64>,
}

fn validate_aspect_ratio(ar: &str) -> &str {
    match ar {
        "9:16" | "1:1" | "4:3" | "16:9" | "original" => ar,
        _ => "9:16",
    }
}

fn normalize_color(c: &str) -> String {
    if c.len() == 7 && c.starts_with('#') {
        c.to_uppercase()
    } else {
        "#FFFFFF".to_string()
    }
}

async fn create_task(
    State(state): State<AppState>,
    Json(req): Json<CreateTaskRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let url = req.url
        .or_else(|| req.source.as_ref().and_then(|s| s.get("url").and_then(|v| v.as_str()).map(|s| s.to_string())))
        .ok_or_else(|| (StatusCode::BAD_REQUEST, Json(json!({"error": "url is required"}))))?;

    let source_type = if url.starts_with("studio://") {
        "studio"
    } else if url.starts_with("novaedit://") || url.starts_with("agentic://") {
        "agentic"
    } else if url.starts_with("repurpose://") {
        "repurpose"
    } else if url.contains("youtube.com") || url.contains("youtu.be") {
        "youtube"
    } else if url.starts_with("upload://") {
        "upload"
    } else {
        "video_url"
    };

    let aspect_ratio = validate_aspect_ratio(
        req.aspect_ratio.as_deref().unwrap_or("9:16")
    ).to_string();

    let font_color = normalize_color(req.font_color.as_deref().unwrap_or("#FFFFFF"));
    let filtered_words_json = serde_json::to_string(
        &req.filtered_words.unwrap_or_default()
    ).unwrap_or_else(|_| "[]".into());

    // Generate UUID in Rust since SQLite has no gen_random_uuid()
    let task_id = Uuid::new_v4();

    let add_subtitles = if req.add_subtitles.unwrap_or(true) { 1i32 } else { 0i32 };
    let include_broll = if req.include_broll.unwrap_or(false) { 1i32 } else { 0i32 };
    let cut_long_pauses = if req.cut_long_pauses.unwrap_or(false) { 1i32 } else { 0i32 };
    let remove_filler_words = if req.remove_filler_words.unwrap_or(false) { 1i32 } else { 0i32 };
    let auto_vertical_reframe = if req.auto_vertical_reframe.unwrap_or(false) { 1i32 } else { 0i32 };
    let reframe_preset = req.reframe_preset.clone().unwrap_or_else(|| "talking_head".into());
    let reframe_frame_skip = req.reframe_frame_skip.unwrap_or(1).clamp(1, 10);
    let reframe_layout = req.reframe_layout.clone().unwrap_or_else(|| "single".into());
    let reframe_layout = match reframe_layout.as_str() {
        "split" | "auto" => reframe_layout,
        _ => "single".to_string(),
    };
    let speaker_active_switch = if req.speaker_active_switch.unwrap_or(false) { 1i32 } else { 0i32 };
    let split_divider = if req.split_divider.unwrap_or(false) { 1i32 } else { 0i32 };
    let originality_boost = req.originality_boost.clone().unwrap_or_else(|| "none".into());
    let translate_language = req.translate_language.clone().unwrap_or_default();
    let stt_provider = req.stt_provider.clone().unwrap_or_else(|| "deepgram".into());
    let studio_payload_json = req.studio_payload.as_ref().map(|p| serde_json::to_string(p).unwrap_or_default());
    let novaedit_payload_json = req.novaedit_payload.as_ref().map(|p| serde_json::to_string(p).unwrap_or_default());
    let repurpose_payload_json = req.repurpose_payload.as_ref().map(|p| serde_json::to_string(p).unwrap_or_default());

    let source_title = req.source_title.unwrap_or_else(|| url.clone());

    let highlight_color = match req.highlight_color.as_deref() {
        Some(c) if c.len() == 7 && c.starts_with('#') => c.to_uppercase(),
        _ => "#FFE000".to_string(),
    };
    let caption_animation = req.caption_animation.clone().unwrap_or_else(|| "word_pop".to_string());
    let auto_emojis = if req.auto_emojis.unwrap_or(false) { 1i32 } else { 0i32 };
    let watermark_position = req.watermark_position.clone().unwrap_or_else(|| "top_right".to_string());
    let watermark_opacity = req.watermark_opacity.unwrap_or(0.8);

    sqlx::query(
        r#"INSERT INTO tasks
           (id, source_url, source_title, source_type, aspect_ratio, num_clips, font_family, font_size,
            font_color, caption_template, add_subtitles, include_broll, processing_mode,
            cut_long_pauses, pause_threshold_ms, remove_filler_words, filtered_words,
             gemini_api_key, deepgram_api_key, openrouter_api_key, stt_provider, auto_vertical_reframe, reframe_preset,
            reframe_frame_skip, reframe_layout, speaker_active_switch, split_divider,
            originality_boost, translate_language, giphy_api_key,
            studio_payload, highlight_color, caption_animation, auto_emojis,
             watermark_position, watermark_opacity, novaedit_payload, llm_provider, repurpose_payload)
            VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"#
    )
    .bind(task_id.to_string())
    .bind(&url)
    .bind(&source_title)
    .bind(source_type)
    .bind(&aspect_ratio)
    .bind(req.num_clips.unwrap_or(5).clamp(1, 30))
    .bind(req.font_family.as_deref().unwrap_or("THEBOLDFONT"))
    .bind(req.font_size.unwrap_or(32).clamp(12, 72))
    .bind(&font_color)
    .bind(req.caption_template.as_deref().unwrap_or("default"))
    .bind(add_subtitles)
    .bind(include_broll)
    .bind(req.processing_mode.as_deref().unwrap_or("fast"))
    .bind(cut_long_pauses)
    .bind(req.pause_threshold_ms.unwrap_or(900).clamp(250, 3000))
    .bind(remove_filler_words)
    .bind(&filtered_words_json)
    .bind(req.gemini_api_key)
    .bind(req.deepgram_api_key)
    .bind(req.openrouter_api_key)
    .bind(&stt_provider)
    .bind(auto_vertical_reframe)
    .bind(&reframe_preset)
    .bind(reframe_frame_skip)
    .bind(&reframe_layout)
    .bind(speaker_active_switch)
    .bind(split_divider)
    .bind(&originality_boost)
    .bind(&translate_language)
    .bind(req.giphy_api_key)
    .bind(studio_payload_json)
    .bind(&highlight_color)
    .bind(&caption_animation)
    .bind(auto_emojis)
    .bind(&watermark_position)
    .bind(watermark_opacity)
    .bind(novaedit_payload_json)
    .bind(req.llm_provider.as_deref().unwrap_or("gemini-3.1-flash-lite"))
    .bind(repurpose_payload_json)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Create task SQL error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()})))
    })?;

    // Push to in-process job queue (replaces Redis LPUSH)
    state.job_tx.send(task_id).await
        .map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"error": "Worker not available"}))))?;

    Ok(Json(json!({
        "task_id": task_id,
        "message": "Task created and queued"
    })))
}

async fn get_task(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let task = sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = ?")
        .bind(id.to_string())
        .fetch_optional(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(json!({"error": "Task not found"}))))?;

    let clips = sqlx::query_as::<_, novaclip_db::GeneratedClip>(
        "SELECT * FROM generated_clips WHERE task_id = ? ORDER BY clip_order ASC"
    )
    .bind(id.to_string())
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let stage_timings: Value = serde_json::from_str(&task.stage_timings).unwrap_or(json!({}));

    Ok(Json(json!({
        "id": task.id,
        "status": task.status,
        "progress": task.progress,
        "progress_message": task.progress_message,
        "source_url": task.source_url,
        "source_title": task.source_title,
        "source_type": task.source_type,
        "aspect_ratio": task.aspect_ratio,
        "num_clips": task.num_clips,
        "font_family": task.font_family,
        "font_size": task.font_size,
        "font_color": task.font_color,
        "caption_template": task.caption_template,
        "add_subtitles": task.add_subtitles,
        "include_broll": task.include_broll,
        "processing_mode": task.processing_mode,
        "auto_vertical_reframe": task.auto_vertical_reframe,
        "reframe_preset": task.reframe_preset,
        "reframe_layout": task.reframe_layout,
        "speaker_active_switch": task.speaker_active_switch,
        "split_divider": task.split_divider,
        "originality_boost": task.originality_boost,
        "stt_provider": task.stt_provider,

        "translate_language": task.translate_language,
        "stage_timings": stage_timings,
        "error_message": task.error_message,
        "studio_payload": task.studio_payload.as_deref().and_then(|s| serde_json::from_str::<Value>(s).ok()),
        "novaedit_payload": task.novaedit_payload.as_deref().and_then(|s| serde_json::from_str::<Value>(s).ok()),
        "edit_plan": task.edit_plan.as_deref().and_then(|s| serde_json::from_str::<Value>(s).ok()),
        "review_score": task.review_score.as_deref().and_then(|s| serde_json::from_str::<Value>(s).ok()),
        "repurpose_payload": task.repurpose_payload.as_deref().and_then(|s| serde_json::from_str::<Value>(s).ok()),
        "repurpose_result": task.repurpose_result.as_deref().and_then(|s| serde_json::from_str::<Value>(s).ok()),
        "highlight_color": task.highlight_color,
        "caption_animation": task.caption_animation,
        "auto_emojis": task.auto_emojis,
        "watermark_position": task.watermark_position,
        "watermark_opacity": task.watermark_opacity,
        "created_at": task.created_at,
        "completed_at": task.completed_at,
        "clips": clips,
    })))
}

async fn repurpose_pdf(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Response, (StatusCode, Json<Value>)> {
    let row: Option<(Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT source_title, repurpose_result FROM tasks WHERE id = ? AND source_type = 'repurpose'"
    ).bind(id.to_string()).fetch_optional(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;
    let (title, raw) = row.ok_or_else(|| (StatusCode::NOT_FOUND, Json(json!({"error": "Repurpose task not found"}))))?;
    let result: Value = serde_json::from_str(raw.as_deref().unwrap_or("{}"))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;
    let mut lines = vec![
        "NOVACLIP - NOVA REPURPOSE".to_string(),
        title.unwrap_or_else(|| "Content Campaign".into()),
        String::new(),
        format!("Audience: {}", result["audience"].as_str().unwrap_or("")),
        format!("Goal: {}", result["goal"].as_str().unwrap_or("")),
        format!("Tone: {}", result["tone"].as_str().unwrap_or("")),
        format!("Core message: {}", result["core_message"].as_str().unwrap_or("")),
        format!("CTA: {}", result["cta"].as_str().unwrap_or("")),
        String::new(),
        "PLATFORM CONTENT".to_string(),
    ];
    if let Some(platforms) = result.get("platform_copy").and_then(Value::as_object) {
        for (platform, content) in platforms {
            lines.push(String::new());
            lines.push(platform.to_uppercase());
            flatten_pdf_value(content, "", &mut lines);
        }
    }
    let pdf = build_text_pdf(&lines);
    Response::builder()
        .header("Content-Type", "application/pdf")
        .header("Content-Disposition", format!("attachment; filename=\"nova-repurpose-{}.pdf\"", id))
        .body(axum::body::Body::from(pdf))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))
}

fn flatten_pdf_value(value: &Value, prefix: &str, lines: &mut Vec<String>) {
    match value {
        Value::Object(map) => for (key, child) in map {
            let label = if prefix.is_empty() { key.replace('_', " ") } else { format!("{} - {}", prefix, key.replace('_', " ")) };
            flatten_pdf_value(child, &label, lines);
        },
        Value::Array(items) => for item in items {
            if let Some(text) = item.as_str() { lines.push(format!("- {}", text)); }
            else { flatten_pdf_value(item, prefix, lines); }
        },
        Value::String(text) => {
            lines.push(format!("{}: {}", prefix, text));
        }
        other if !other.is_null() => lines.push(format!("{}: {}", prefix, other)),
        _ => {}
    }
}

fn build_text_pdf(lines: &[String]) -> Vec<u8> {
    fn escape(text: &str) -> String {
        text.replace('\\', "\\\\").replace('(', "\\(").replace(')', "\\)")
            .chars().map(|c| if c.is_ascii() { c } else { '?' }).collect()
    }
    let mut content = String::from("BT\n/F1 11 Tf\n50 790 Td\n14 TL\n");
    for line in lines.iter().take(52) {
        let mut remaining = line.as_str();
        while !remaining.is_empty() {
            let end = remaining.char_indices().nth(92).map(|(i, _)| i).unwrap_or(remaining.len());
            content.push_str(&format!("({}) Tj\nT*\n", escape(&remaining[..end])));
            remaining = &remaining[end..];
        }
        if line.is_empty() { content.push_str("T*\n"); }
    }
    content.push_str("ET\n");
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 842] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>".to_string(),
        format!("<< /Length {} >>\nstream\n{}endstream", content.len(), content),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ];
    let mut pdf = String::from("%PDF-1.4\n");
    let mut offsets = vec![0usize];
    for (i, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.push_str(&format!("{} 0 obj\n{}\nendobj\n", i + 1, object));
    }
    let xref = pdf.len();
    pdf.push_str(&format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1));
    for offset in offsets.iter().skip(1) { pdf.push_str(&format!("{:010} 00000 n \n", offset)); }
    pdf.push_str(&format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF", objects.len() + 1, xref));
    pdf.into_bytes()
}

async fn delete_task(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    sqlx::query("DELETE FROM tasks WHERE id = ?")
        .bind(id.to_string())
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;
    Ok(Json(json!({"message": "Task deleted"})))
}

#[derive(Deserialize)]
struct UpdateTaskRequest { title: Option<String> }

async fn update_task(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateTaskRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if let Some(title) = req.title {
        sqlx::query("UPDATE tasks SET source_title = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(&title).bind(id.to_string()).execute(&state.db).await.ok();
    }
    Ok(Json(json!({"message": "Updated"})))
}

const AI_PROMPT_SYSTEM_PROMPT: &str = r##"You are a task parameter parser for a video clipping app called NovaClip. Given a user instruction and a video URL, extract the user's intent and return ONLY valid JSON with these fields (all optional, use defaults if not specified):

{
  "num_clips": 5,
  "aspect_ratio": "9:16",
  "add_subtitles": true,
  "auto_vertical_reframe": false,
  "reframe_preset": "talking_head",
  "caption_template": "default",
  "originality_boost": "none",
  "auto_memes": false,
  "translate_language": "",
  "reasoning": "brief explanation of what the user asked for"
}

Rules:
- num_clips: 1-30. Default 5.
- aspect_ratio: "9:16", "1:1", "16:9", "original". Default "9:16".
- reframe_preset: "talking_head", "sports", "pets", "cars". Only used if auto_vertical_reframe is true.
- caption_template: "default", "bold", "vibrant", "tiktok", "neon", "podcast", "minimal", "cinematic", "cyber", "clean"
- originality_boost: "none", "light", "balanced", "strong"
- translate_language: "" (none), "ko", "ja", "zh", "es", "fr", "de", "pt"
- "add_subtitles": true unless user says no captions
- "auto_memes": true only if user explicitly asks for memes/reactions/GIFs
- "auto_vertical_reframe": true if user mentions vertical, vertical crop, subject tracking, or reframe

Return ONLY valid JSON, no markdown, no explanations."##;

#[derive(Deserialize)]
struct AiPromptRequest {
    url: String,
    instruction: String,
    gemini_api_key: Option<String>,
}

async fn ai_prompt_handler(
    State(state): State<AppState>,
    Json(req): Json<AiPromptRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let gemini_key = req.gemini_api_key
        .filter(|k| !k.trim().is_empty())
        .or_else(|| std::env::var("GEMINI_API_KEY").ok().filter(|k| !k.trim().is_empty()))
        .ok_or_else(|| (StatusCode::BAD_REQUEST, Json(json!({"error": "Gemini API key required"}))))?;

    let gemini_model = std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-3.1-flash-lite".into());
    let user_prompt = format!("Video URL: {}\nUser instruction: {}", req.url, req.instruction);

    let body = json!({
        "systemInstruction": {"parts": [{"text": AI_PROMPT_SYSTEM_PROMPT}]},
        "contents": [{"role": "user", "parts": [{"text": user_prompt}]}],
        "generationConfig": {"temperature": 0.2, "maxOutputTokens": 1024, "responseMimeType": "application/json"}
    });

    let url = format!("https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}", gemini_model, gemini_key);
    let resp = reqwest::Client::new().post(&url).json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send().await
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(json!({"error": format!("Gemini error: {}", e)}))))?;

    let text = resp.json::<Value>().await
        .ok().and_then(|r| r.pointer("/candidates/0/content/parts/0/text").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .ok_or_else(|| (StatusCode::BAD_GATEWAY, Json(json!({"error": "Empty Gemini response"}))))?;

    let clean = text.trim().trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim();
    let params: Value = serde_json::from_str(clean)
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(json!({"error": format!("Parse error: {}", e)}))))?;

    // Determine source type
    let source_type = if req.url.contains("youtube.com") || req.url.contains("youtu.be") { "youtube" }
        else if req.url.starts_with("upload://") { "upload" } else { "video_url" };

    let task_id = Uuid::new_v4();
    let num_clips = params["num_clips"].as_i64().unwrap_or(5).clamp(1, 30) as i32;
    let aspect_ratio = match params["aspect_ratio"].as_str().unwrap_or("9:16") { "1:1" => "1:1", "16:9" => "16:9", "original" => "original", _ => "9:16" };
    let add_subtitles = if params["add_subtitles"].as_bool().unwrap_or(true) { 1i32 } else { 0i32 };
    let auto_vertical_reframe = if params["auto_vertical_reframe"].as_bool().unwrap_or(false) { 1i32 } else { 0i32 };
    let reframe_preset = params["reframe_preset"].as_str().unwrap_or("talking_head");
    let reframe_layout = match params["reframe_layout"].as_str().unwrap_or("single") {
        "split" | "auto" => "split",
        _ => "single",
    };
    let speaker_active_switch = if params["speaker_active_switch"].as_bool().unwrap_or(false) { 1i32 } else { 0i32 };
    let split_divider = if params["split_divider"].as_bool().unwrap_or(false) { 1i32 } else { 0i32 };
    let caption_template = params["caption_template"].as_str().unwrap_or("default");
    let originality_boost = params["originality_boost"].as_str().unwrap_or("none");
    let translate_language = params["translate_language"].as_str().unwrap_or("");

    let stt_provider = params["stt_provider"].as_str().unwrap_or("deepgram");

    sqlx::query(
        r#"INSERT INTO tasks
           (id, source_url, source_type, aspect_ratio, num_clips, font_family, font_size,
            font_color, caption_template, add_subtitles, include_broll, processing_mode,
            cut_long_pauses, pause_threshold_ms, remove_filler_words, filtered_words,
            gemini_api_key, stt_provider, auto_vertical_reframe, reframe_preset, reframe_frame_skip,
            reframe_layout, speaker_active_switch, split_divider,
            originality_boost, translate_language)
           VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"#
    )
    .bind(task_id.to_string())
    .bind(&req.url).bind(source_type).bind(aspect_ratio).bind(num_clips)
    .bind("THEBOLDFONT").bind(32).bind("#FFFFFF").bind(caption_template)
    .bind(add_subtitles).bind(0).bind("fast").bind(0).bind(900).bind(0).bind("[]")
    .bind(&gemini_key).bind(stt_provider)
    .bind(auto_vertical_reframe).bind(reframe_preset).bind(1)
    .bind(reframe_layout).bind(speaker_active_switch).bind(split_divider)
    .bind(originality_boost).bind(translate_language)
    .execute(&state.db).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;

    state.job_tx.send(task_id).await.ok();

    Ok(Json(json!({
        "task_id": task_id,
        "message": "Task created from AI prompt",
        "params": params,
    })))
}

#[derive(Deserialize)]
struct AiChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AiChatRequest {
    url: String,
    params: Option<Value>,
    messages: Vec<AiChatMessage>,
    gemini_api_key: Option<String>,
}

const AI_CHAT_SYSTEM_PROMPT: &str = r##"You are Nova, the friendly conversational AI video editor inside NovaClip. The user pasted a video URL and wants you to turn it into viral short clips. Hold a short conversation to lock in the clip settings BEFORE any processing starts, then hand off to the clipping pipeline.

The video URL is: URL_PLACEHOLDER

Settings already decided so far (may be partial or empty): PARAMS_PLACEHOLDER

Gather, ONE question at a time, any of these that are still missing (never re-ask something already decided unless the user changes their mind):
1. Orientation (field "aspect_ratio"): "9:16" vertical, "1:1" square, "16:9" widescreen, "original" (no crop)
2. Number of clips (field "num_clips"): whole number 1-30
3. Burn-in karaoke captions (field "add_subtitles"): true (yes) or false (no)
4. Caption language (field "translate_language"): "" for the original audio language, otherwise one of en, es, fr, de, it, pt, nl, ru, zh, zh-TW, ja, ko, ar, hi, bn, tr, vi, th, id, pl, uk, sv
5. Caption style (field "caption_template"): "default", "bold", "vibrant", "tiktok", "neon", "podcast", "minimal", "cinematic", "cyber", "clean"
6. AI subject-tracking reframe (field "auto_vertical_reframe"): true/false — only relevant when orientation is 9:16
7. Originality boost (field "originality_boost"): "none", "light", "balanced", "strong"

Keep questions casual and short, like a helpful friend, and offer 2-5 clickable options per question. If the user says "you decide", "just do it", "surprise me", etc., stop asking and pick smart defaults.

When every setting is resolved, respond with type "ready", the complete params object, and a 1-2 sentence summary of what will be produced.

Respond ONLY with valid JSON matching exactly one of these two schemas:

Question (you still need more info):
{"type": "question", "field": "aspect_ratio", "question": "What orientation should the clips be in?", "options": [{"label": "Vertical 9:16", "value": "9:16"}, {"label": "Square 1:1", "value": "1:1"}, {"label": "Widescreen 16:9", "value": "16:9"}, {"label": "Original (no crop)", "value": "original"}], "params": {"aspect_ratio": "9:16"}}

Ready (everything is locked in):
{"type": "ready", "params": {"aspect_ratio": "9:16", "num_clips": 5, "add_subtitles": true, "caption_template": "default", "translate_language": "", "auto_vertical_reframe": true, "reframe_preset": "talking_head", "originality_boost": "none"}, "summary": "I'll cut 5 vertical shorts with AI subject tracking and burned-in karaoke captions."}

Rules:
- option labels are human-readable; option values and params values must be exact machine values listed above.
- Always include "field" and the running "params" object on question responses. The "params" object in question responses should merge in anything just decided.
- Never ask for or mention the video URL — it is already set.
- If the user's last message is off-topic, respond with type "question", field "general", and a friendly question steering back to the clip settings."##;

async fn ai_chat_handler(
    State(state): State<AppState>,
    Json(req): Json<AiChatRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let gemini_key = req.gemini_api_key
        .filter(|k| !k.trim().is_empty())
        .or_else(|| std::env::var("GEMINI_API_KEY").ok().filter(|k| !k.trim().is_empty()))
        .ok_or_else(|| (StatusCode::BAD_REQUEST, Json(json!({"error": "Gemini API key required"}))))?;

    let gemini_model = std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-3.1-flash-lite".into());

    if req.messages.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "messages cannot be empty"}))));
    }

    let decided = req.params.unwrap_or(json!({}));
    let system_prompt = AI_CHAT_SYSTEM_PROMPT
        .replace("URL_PLACEHOLDER", &req.url)
        .replace("PARAMS_PLACEHOLDER", &decided.to_string());

    let contents: Vec<Value> = req.messages.iter().map(|m| {
        json!({
            "role": if m.role.eq_ignore_ascii_case("assistant") { "model" } else { "user" },
            "parts": [{"text": m.content}]
        })
    }).collect();

    let body = json!({
        "systemInstruction": {"parts": [{"text": system_prompt}]},
        "contents": contents,
        "generationConfig": {"temperature": 0.3, "maxOutputTokens": 2048, "responseMimeType": "application/json"}
    });

    let url = format!("https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}", gemini_model, gemini_key);
    let resp = reqwest::Client::new().post(&url).json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send().await
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(json!({"error": format!("Gemini error: {}", e)}))))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err((StatusCode::BAD_GATEWAY, Json(json!({"error": format!("Gemini error {}: {}", status, text)}))));
    }

    let text = resp.json::<Value>().await
        .ok().and_then(|r| r.pointer("/candidates/0/content/parts/0/text").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .ok_or_else(|| (StatusCode::BAD_GATEWAY, Json(json!({"error": "Empty Gemini response"}))))?;

    let clean = text.trim().trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim();
    let parsed: Value = serde_json::from_str(clean)
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(json!({"error": format!("Parse error: {}", e)}))))?;

    Ok(Json(parsed))
}

/// SSE progress stream — polls the tasks table every second instead of Redis pub/sub.
/// Latency: ≤1s per update (acceptable for long-running video processing).
async fn task_progress_sse(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        let id_str = id.to_string();
        let mut last_progress = -1i32;
        let mut last_status = String::new();
        let mut last_message = String::new();
        let mut interval = tokio::time::interval(Duration::from_secs(1));

        loop {
            interval.tick().await;

    let task = sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = ?")
                .bind(&id_str)
                .fetch_optional(&state.db)
                .await;

            match task {
                Ok(Some(t)) => {
                    let msg = t.progress_message.clone().unwrap_or_default();
                    let changed = t.progress != last_progress
                        || t.status != last_status
                        || msg != last_message;

                    if changed {
                        last_progress = t.progress;
                        last_status = t.status.clone();
                        last_message = msg;

                        let payload = json!({
                            "task_id": id,
                            "percent": t.progress,
                            "message": t.progress_message,
                            "status": t.status,
                            "event_type": "progress",
                        });
                        yield Ok(Event::default().event("progress").data(payload.to_string()));

                        if t.status == "completed" || t.status == "error" || t.status == "cancelled" {
                            break;
                        }
                    }
                }
                Ok(None) => {
                    yield Ok(Event::default().event("error").data(
                        json!({"error": "Task not found"}).to_string()
                    ));
                    break;
                }
                Err(_) => {
                    // DB error — wait a bit and retry
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

async fn cancel_task(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Sets cancel_requested flag — the worker polls this between pipeline stages
    sqlx::query(
        "UPDATE tasks SET cancel_requested = 1, status = 'cancelled', \
         progress_message = 'Cancelled by user', updated_at = datetime('now') WHERE id = ?"
    )
    .bind(id.to_string())
    .execute(&state.db)
    .await
    .ok();
    Ok(Json(json!({"message": "Task cancelled"})))
}

async fn resume_task(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let _task = sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = ?")
        .bind(id.to_string()).fetch_optional(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(json!({"error": "Task not found"}))))?;

    sqlx::query(
        "UPDATE tasks SET status = 'queued', cancel_requested = 0, progress = 0, \
         progress_message = 'Re-queued', updated_at = datetime('now') WHERE id = ?"
    )
    .bind(id.to_string()).execute(&state.db).await.ok();

    // Re-push to in-process queue
    state.job_tx.send(id).await.ok();

    Ok(Json(json!({"message": "Task resumed"})))
}

#[derive(Deserialize)]
struct ApplySettingsRequest {
    font_family: Option<String>,
    font_size: Option<i32>,
    font_color: Option<String>,
    caption_template: Option<String>,
    include_broll: Option<bool>,
    add_subtitles: Option<bool>,
    cut_long_pauses: Option<bool>,
    pause_threshold_ms: Option<i32>,
    remove_filler_words: Option<bool>,
    filtered_words: Option<Vec<String>>,
    auto_vertical_reframe: Option<bool>,
    reframe_preset: Option<String>,
    reframe_layout: Option<String>,
    speaker_active_switch: Option<bool>,
    split_divider: Option<bool>,
    originality_boost: Option<String>,
    translate_language: Option<String>,
    stt_provider: Option<String>,
}

async fn apply_settings(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<ApplySettingsRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let font_color = normalize_color(req.font_color.as_deref().unwrap_or("#FFFFFF"));
    let reframe_layout = req.reframe_layout.clone().filter(|v| v == "split" || v == "auto");
    let speaker_active_switch = req.speaker_active_switch.map(|v| if v { 1 } else { 0 });
    let split_divider = req.split_divider.map(|v| if v { 1 } else { 0 });
    sqlx::query(
        r#"UPDATE tasks SET
            font_family = COALESCE(?, font_family),
            font_size = COALESCE(?, font_size),
            font_color = ?,
            caption_template = COALESCE(?, caption_template),
            include_broll = COALESCE(?, include_broll),
            add_subtitles = COALESCE(?, add_subtitles),
            cut_long_pauses = COALESCE(?, cut_long_pauses),
            pause_threshold_ms = COALESCE(?, pause_threshold_ms),
            remove_filler_words = COALESCE(?, remove_filler_words),
            auto_vertical_reframe = COALESCE(?, auto_vertical_reframe),
            reframe_preset = COALESCE(?, reframe_preset),
            reframe_layout = COALESCE(?, reframe_layout),
            speaker_active_switch = COALESCE(?, speaker_active_switch),
            split_divider = COALESCE(?, split_divider),
            originality_boost = COALESCE(?, originality_boost),
            translate_language = COALESCE(?, translate_language),
            stt_provider = COALESCE(?, stt_provider),
            updated_at = datetime('now')
            WHERE id = ?"#
    )
    .bind(req.font_family)
    .bind(req.font_size)
    .bind(&font_color)
    .bind(req.caption_template)
    .bind(req.include_broll)
    .bind(req.add_subtitles)
    .bind(req.cut_long_pauses)
    .bind(req.pause_threshold_ms)
    .bind(req.remove_filler_words)
    .bind(req.auto_vertical_reframe)
    .bind(req.reframe_preset)
    .bind(reframe_layout)
    .bind(speaker_active_switch)
    .bind(split_divider)
    .bind(req.originality_boost)
    .bind(req.translate_language)
    .bind(req.stt_provider)
    .bind(id.to_string())
    .execute(&state.db)
    .await
    .ok();
    Ok(Json(json!({"message": "Settings applied"})))
}

async fn delete_clip(
    State(state): State<AppState>,
    Path((task_id, clip_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    sqlx::query("DELETE FROM generated_clips WHERE id = ? AND task_id = ?")
        .bind(clip_id.to_string())
        .bind(task_id.to_string())
        .execute(&state.db)
        .await
        .ok();
    Ok(Json(json!({"message": "Clip deleted"})))
}

#[derive(Deserialize)]
struct TrimRequest { start_offset: Option<f64>, end_offset: Option<f64> }

async fn trim_clip(
    State(state): State<AppState>,
    Path((task_id, clip_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<TrimRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let clip = sqlx::query_as::<_, novaclip_db::GeneratedClip>(
        "SELECT * FROM generated_clips WHERE id = ? AND task_id = ?"
    )
    .bind(clip_id.to_string())
    .bind(task_id.to_string())
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?
    .ok_or_else(|| (StatusCode::NOT_FOUND, Json(json!({"error": "Clip not found"}))))?;

    let start_offset = req.start_offset.unwrap_or(0.0).max(0.0);
    let end_offset = req.end_offset.unwrap_or(0.0).max(0.0);
    let input = std::path::Path::new(&clip.file_path);
    let output_dir = input.parent().unwrap_or(std::path::Path::new("."));
    let output_name = format!("trim_{}.mp4", uuid::Uuid::new_v4().to_string().replace("-", ""));
    let output_path = output_dir.join(&output_name);

    let duration = clip.duration - start_offset - end_offset;
    if duration <= 0.0 {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Trim would produce empty clip"}))));
    }

    let status = tokio::process::Command::new("ffmpeg")
        .args(["-y", "-ss", &start_offset.to_string(), "-i", &clip.file_path,
               "-t", &duration.to_string(), "-c:v", "libx264", "-crf", "18",
               "-preset", "fast", "-c:a", "aac", output_path.to_str().unwrap()])
        .status().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;

    if !status.success() {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "FFmpeg trim failed"}))));
    }

    sqlx::query(
        "UPDATE generated_clips SET file_path = ?, filename = ?, duration = ?, updated_at = datetime('now') WHERE id = ?"
    )
    .bind(output_path.to_str().unwrap())
    .bind(&output_name)
    .bind(duration)
    .bind(clip_id.to_string())
    .execute(&state.db)
    .await
    .ok();

    Ok(Json(json!({"message": "Clip trimmed", "duration": duration})))
}

#[derive(Deserialize)]
struct SplitRequest { split_time: f64 }

async fn split_clip(
    State(_state): State<AppState>,
    Path((_task_id, _clip_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<SplitRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    Ok(Json(json!({"message": "Split queued", "split_time": req.split_time})))
}

async fn regenerate_clip(
    State(_state): State<AppState>,
    Path((_task_id, _clip_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    Ok(Json(json!({"message": "Regenerate queued"})))
}

#[derive(Deserialize)]
struct CaptionsRequest {
    caption_text: Option<String>,
    position: Option<String>,
    highlight_words: Option<Vec<String>>,
}

async fn update_captions(
    State(_state): State<AppState>,
    Path((_task_id, _clip_id)): Path<(Uuid, Uuid)>,
    Json(_req): Json<CaptionsRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    Ok(Json(json!({"message": "Captions updated"})))
}

#[derive(Deserialize)]
struct MergeRequest { clip_ids: Vec<Uuid> }

async fn merge_clips(
    State(_state): State<AppState>,
    Path(_task_id): Path<Uuid>,
    Json(req): Json<MergeRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    Ok(Json(json!({"message": "Merge queued", "clip_ids": req.clip_ids})))
}

async fn download_all_clips(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, (StatusCode, Json<Value>)> {
    let clips = sqlx::query_as::<_, GeneratedClip>(
        "SELECT * FROM generated_clips WHERE task_id = ? ORDER BY clip_order ASC"
    )
    .bind(&id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;

    if clips.is_empty() {
        return Err((StatusCode::NOT_FOUND, Json(json!({"error": "No clips found for this task"}))));
    }

    let mut zip_buf = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_buf));
        let options = zip::write::FileOptions::<()>::default()
            .compression_method(zip::CompressionMethod::Stored);

        let output_dir = std::path::PathBuf::from(&state.config.output_dir).join(&id);

        for (i, clip) in clips.iter().enumerate() {
            let file_path = output_dir.join(&clip.filename);
            if file_path.exists() {
                if let Ok(file_data) = std::fs::read(&file_path) {
                    let zip_filename = format!("clip_{:02}_{}.mp4", i + 1, &clip.id[..8.min(clip.id.len())]);
                    zip.start_file(zip_filename, options).ok();
                    std::io::Write::write_all(&mut zip, &file_data).ok();
                }
            }
        }
        zip.finish().ok();
    }

    let response = Response::builder()
        .header("Content-Type", "application/zip")
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"novaclip_{}.zip\"", &id[..8.min(id.len())]),
        )
        .body(axum::body::Body::from(zip_buf))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;

    Ok(response)
}

#[derive(Deserialize)]
struct StudioScriptReq {
    topic: String,
    vibe: Option<String>,
    duration: Option<i32>,
    llm_provider: Option<String>,
    api_key: Option<String>,
}

async fn generate_studio_script(
    Json(req): Json<StudioScriptReq>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    use novaclip_worker::pipeline::studio_llm::StudioLlmProcessor;

    let topic = req.topic.trim();
    if topic.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "topic is required"}))));
    }

    let vibe = req.vibe.unwrap_or_else(|| "aesthetic".into());
    let provider = req.llm_provider.unwrap_or_else(|| "gemini-3.1-flash-lite".into());

    let openrouter_key = std::env::var("OPENROUTER_API_KEY").ok()
        .filter(|k| !k.trim().is_empty())
        .or_else(|| req.api_key.clone().filter(|k| k.starts_with("sk-or-")))
        .unwrap_or_default();

    let gemini_key = req.api_key
        .filter(|k| !k.trim().is_empty() && !k.starts_with("sk-or-"))
        .or_else(|| std::env::var("GEMINI_API_KEY").ok())
        .unwrap_or_default();

    let is_openrouter = provider.contains('/') || provider == "openrouter/free";

    if is_openrouter && openrouter_key.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "OpenRouter API key required — set one in Settings or add OPENROUTER_API_KEY to .env"}))));
    } else if !is_openrouter && gemini_key.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Gemini API key required — set one in Settings or add GEMINI_API_KEY to .env"}))));
    }

    let duration = req.duration.unwrap_or(60);
    let processor = StudioLlmProcessor::new(gemini_key, provider, openrouter_key);
    let script = processor.generate_topic_script(topic, &vibe, duration)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;

    if script.trim().is_empty() {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "AI returned an empty script — check your API key and model name"}))));
    }

    Ok(Json(json!({ "script": script })))
}

#[derive(Deserialize)]
struct ApproveEditPlanRequest {
    #[serde(default)]
    edit_plan: Option<Value>,
}

/// Approve (and optionally edit) the Director's edit plan, then resume rendering.
async fn approve_edit_plan(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<ApproveEditPlanRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let id_str = id.to_string();

    let row: Option<(String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT source_type, novaedit_payload, edit_plan FROM tasks WHERE id = ?"
    )
    .bind(&id_str)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;

    let (source_type, payload_raw, current_plan) = row
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(json!({"error": "Task not found"}))))?;

    if source_type != "agentic" {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Not an agentic edit task"}))));
    }

    // User may have edited the plan — validate and store it
    if let Some(new_plan) = req.edit_plan {
        let mut plan: novaclip_worker::pipeline::nova_edit::NovaEditPlan = serde_json::from_value(new_plan.clone())
            .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": format!("Invalid edit plan: {}", e)}))))?;
        let total: f64 = plan.entries.iter().map(|en| (en.end_trim - en.start_trim).max(0.0)).sum();
        plan.total_duration = total;
        sqlx::query("UPDATE tasks SET edit_plan = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(serde_json::to_string(&plan).unwrap_or_default())
            .bind(&id_str)
            .execute(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;
    } else if current_plan.is_none() {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "No edit plan exists to approve"}))));
    }

    // Move payload stage → editing
    if let Some(raw) = payload_raw {
        let mut payload: Value = serde_json::from_str(&raw).unwrap_or(json!({}));
        payload["stage"] = json!("editing");
        if payload["retries_used"].is_null() {
            payload["retries_used"] = json!(0);
        }
        sqlx::query("UPDATE tasks SET novaedit_payload = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(serde_json::to_string(&payload).unwrap_or_default())
            .bind(&id_str)
            .execute(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;
    }

    sqlx::query(
        "UPDATE tasks SET status = 'queued', progress = 50, progress_message = 'Edit approved — rendering...', error_message = NULL, completed_at = NULL, updated_at = datetime('now') WHERE id = ?"
    )
    .bind(&id_str)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;

    state.job_tx.send(id).await
        .map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"error": "Worker not available"}))))?;

    Ok(Json(json!({"task_id": id_str, "message": "Edit plan approved — rendering started"})))
}

#[derive(Deserialize)]
struct ReplanRequest {
    message: String,
}

/// Re-run the agentic edit with human feedback (after completion or before approval).
async fn replan_task(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<ReplanRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let message = req.message.trim().to_string();
    if message.is_empty() || message.chars().count() > 4000 {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Feedback message must be 1-4000 characters"}))));
    }

    let id_str = id.to_string();

    let row: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT source_type, novaedit_payload FROM tasks WHERE id = ?"
    )
    .bind(&id_str)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;

    let (source_type, payload_raw) = row
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(json!({"error": "Task not found"}))))?;

    if source_type != "agentic" {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Not an agentic edit task"}))));
    }

    let mut payload: Value = payload_raw
        .and_then(|r| serde_json::from_str(&r).ok())
        .unwrap_or(json!({}));
    payload["stage"] = json!("editing");
    payload["retries_used"] = json!(0);
    payload["user_feedback"] = json!(message.clone());
    if payload["feedback_history"].as_array().is_none() {
        payload["feedback_history"] = json!([]);
    }
    if let Some(arr) = payload["feedback_history"].as_array_mut() {
        if !arr.iter().any(|v| v.as_str() == Some(&message)) {
            arr.push(json!(message.clone()));
        }
    }

    sqlx::query("UPDATE tasks SET novaedit_payload = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(serde_json::to_string(&payload).unwrap_or_default())
        .bind(&id_str)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;

    sqlx::query(
        "UPDATE tasks SET status = 'queued', progress = 50, progress_message = 'Re-planning with your feedback...', error_message = NULL, completed_at = NULL, updated_at = datetime('now') WHERE id = ?"
    )
    .bind(&id_str)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;

    state.job_tx.send(id).await
        .map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"error": "Worker not available"}))))?;

    Ok(Json(json!({"task_id": id_str, "message": "Re-planning started with your feedback"})))
}

async fn upload_watermark(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let id_str = id.to_string();

    // Verify task exists
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE id = ?")
        .bind(&id_str)
        .fetch_one(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;

    if count == 0 {
        return Err((StatusCode::NOT_FOUND, Json(json!({"error": "Task not found"}))));
    }

    let output_dir = PathBuf::from(
        std::env::var("OUTPUT_DIR").unwrap_or_else(|_| "outputs".into())
    ).join(&id_str);
    tokio::fs::create_dir_all(&output_dir).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to create directory: {}", e)}))))?;

    let mut saved_path: Option<String> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "watermark" {
            let file_name = field.file_name().unwrap_or("watermark.png").to_string();
            let p = PathBuf::from(&file_name);
            let ext = p.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("png");
            let dest = output_dir.join(format!("watermark.{}", ext));
            let data = field.bytes().await
                .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": format!("Failed to read file: {}", e)}))))?;

            tokio::fs::write(&dest, &data).await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to save file: {}", e)}))))?;

            saved_path = Some(dest.to_string_lossy().to_string());
            break;
        }
    }

    match saved_path {
        Some(path) => {
            sqlx::query("UPDATE tasks SET watermark_path = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(&path)
                .bind(&id_str)
                .execute(&state.db)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;
            Ok(Json(json!({"message": "Watermark uploaded", "path": path})))
        }
        None => Err((StatusCode::BAD_REQUEST, Json(json!({"error": "No watermark file found in upload. Use field name 'watermark'."})))),
    }
}
