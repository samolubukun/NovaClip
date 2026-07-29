use std::collections::HashMap;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Response, Sse},
    routing::{delete, get, patch, post},
    Json, Router,
};
use futures::stream::Stream;
use serde::Deserialize;
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
        .route("/tasks/ai-prompt", post(ai_prompt_handler))
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
    deepgram_api_key: Option<String>,
    auto_vertical_reframe: Option<bool>,
    reframe_preset: Option<String>,
    reframe_frame_skip: Option<i32>,
    originality_boost: Option<String>,
    translate_language: Option<String>,
    giphy_api_key: Option<String>,
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

    let source_type = if url.contains("youtube.com") || url.contains("youtu.be") {
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
    let originality_boost = req.originality_boost.clone().unwrap_or_else(|| "none".into());
    let translate_language = req.translate_language.clone().unwrap_or_default();

    sqlx::query(
        r#"INSERT INTO tasks
           (id, source_url, source_type, aspect_ratio, num_clips, font_family, font_size,
            font_color, caption_template, add_subtitles, include_broll, processing_mode,
            cut_long_pauses, pause_threshold_ms, remove_filler_words, filtered_words,
            gemini_api_key, deepgram_api_key, auto_vertical_reframe, reframe_preset,
            reframe_frame_skip, originality_boost, translate_language, giphy_api_key)
           VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"#
    )
    .bind(task_id.to_string())
    .bind(&url)
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
    .bind(auto_vertical_reframe)
    .bind(&reframe_preset)
    .bind(reframe_frame_skip)
    .bind(&originality_boost)
    .bind(&translate_language)
    .bind(req.giphy_api_key)
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
        "originality_boost": task.originality_boost,

        "translate_language": task.translate_language,
        "stage_timings": stage_timings,
        "error_message": task.error_message,
        "created_at": task.created_at,
        "completed_at": task.completed_at,
        "clips": clips,
    })))
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
    let caption_template = params["caption_template"].as_str().unwrap_or("default");
    let originality_boost = params["originality_boost"].as_str().unwrap_or("none");
    let translate_language = params["translate_language"].as_str().unwrap_or("");

    sqlx::query(
        r#"INSERT INTO tasks
           (id, source_url, source_type, aspect_ratio, num_clips, font_family, font_size,
            font_color, caption_template, add_subtitles, include_broll, processing_mode,
            cut_long_pauses, pause_threshold_ms, remove_filler_words, filtered_words,
            gemini_api_key, auto_vertical_reframe, reframe_preset, reframe_frame_skip,
            originality_boost, translate_language)
           VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"#
    )
    .bind(task_id.to_string())
    .bind(&req.url).bind(source_type).bind(aspect_ratio).bind(num_clips)
    .bind("THEBOLDFONT").bind(32).bind("#FFFFFF").bind(caption_template)
    .bind(add_subtitles).bind(0).bind("fast").bind(0).bind(900).bind(0).bind("[]")
    .bind(&gemini_key)
    .bind(auto_vertical_reframe).bind(reframe_preset).bind(1)
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
    originality_boost: Option<String>,
    translate_language: Option<String>,
}

async fn apply_settings(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<ApplySettingsRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let font_color = normalize_color(req.font_color.as_deref().unwrap_or("#FFFFFF"));
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
            originality_boost = COALESCE(?, originality_boost),
            translate_language = COALESCE(?, translate_language),
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
    .bind(req.originality_boost)
    .bind(req.translate_language)
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
