use axum::{
    extract::State,
    http::StatusCode,
    routing::post,
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;
use uuid::Uuid;
use crate::routes::youtube_studio::parse_json_block;
use crate::state::AppState;

pub fn mcp_router() -> Router<AppState> {
    Router::new()
        .route("/mcp", post(mcp_handler))
}

/// MCP (Model Context Protocol) JSON-RPC endpoint.
/// Enables Claude, ChatGPT, Cursor, and other MCP clients to control the NovaClip pipeline.
/// Tools: process_video, get_job_status, list_clips, get_quota, add_subtitles, publish_clip
#[derive(Deserialize)]
struct McpRequest {
    #[allow(dead_code)]
    jsonrpc: Option<String>,
    method: String,
    params: Option<Value>,
    id: Option<Value>,
}

async fn mcp_handler(
    State(state): State<AppState>,
    Json(req): Json<McpRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let id = req.id.unwrap_or(Value::Null);

    match req.method.as_str() {
        "initialize" => Ok(Json(mcp_response(id, json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "NovaClip MCP Server",
                "version": "1.0.0"
            }
        })))),

        "notifications/initialized" => Ok(Json(json!({}))),

        "tools/list" => Ok(Json(mcp_response(id, json!({
            "tools": [
                {
                    "name": "process_video",
                    "description": "Submit a video URL for clipping into viral shorts. Returns a task ID for status tracking.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "url": {"type": "string", "description": "YouTube URL or uploaded video path"},
                            "num_clips": {"type": "integer", "description": "Number of clips to extract (1-30)", "default": 5},
                            "aspect_ratio": {"type": "string", "enum": ["9:16", "1:1", "16:9", "original"], "default": "9:16"},
                            "add_subtitles": {"type": "boolean", "default": true}
                        },
                        "required": ["url"]
                    }
                },
                {
                    "name": "get_job_status",
                    "description": "Get the current status and progress of a processing job.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "job_id": {"type": "string", "description": "Task UUID"}
                        },
                        "required": ["job_id"]
                    }
                },
                {
                    "name": "list_clips",
                    "description": "List all generated clips for a completed task with their URLs.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "job_id": {"type": "string", "description": "Task UUID"}
                        },
                        "required": ["job_id"]
                    }
                },
                {
                    "name": "get_quota",
                    "description": "Return the current number of tasks in the system (simple quota check).",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                {
                    "name": "add_subtitles",
                    "description": "Burn styled captions onto an existing clip. Re-encodes the clip with new subtitle styling (style, position, font, color).",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "job_id": {"type": "string", "description": "Task UUID"},
                            "clip_id": {"type": "string", "description": "Clip UUID (from list_clips)"},
                            "clip_index": {"type": "integer", "description": "0-based clip index within the job (used when clip_id is omitted)", "default": 0},
                            "style": {"type": "string", "enum": ["classic", "karaoke"], "description": "Subtitle style", "default": "classic"},
                            "position": {"type": "string", "enum": ["top", "middle", "bottom"], "description": "Caption position on screen", "default": "bottom"},
                            "font_size": {"type": "integer", "description": "Caption font size in pixels", "default": 48},
                            "font_family": {"type": "string", "default": "THEBOLDFONT"},
                            "font_color": {"type": "string", "description": "Hex color like #FFFFFF", "default": "#FFFFFF"},
                            "highlight_color": {"type": "string", "description": "Hex accent color, used as the caption color in karaoke style", "default": "#FFFF00"},
                            "uppercase": {"type": "boolean", "default": false}
                        },
                        "required": ["job_id"]
                    }
                },
                {
                    "name": "publish_clip",
                    "description": "Publish a generated clip to social platforms via Upload-Post (YouTube, TikTok, Instagram).",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "job_id": {"type": "string", "description": "Task UUID"},
                            "clip_id": {"type": "string", "description": "Clip UUID (from list_clips)"},
                            "clip_index": {"type": "integer", "description": "0-based clip index within the job (used when clip_id is omitted)", "default": 0},
                            "api_key": {"type": "string", "description": "Upload-Post API key (falls back to UPLOADPOST_API_KEY env)"},
                            "profile": {"type": "string", "description": "Upload-Post profile username (auto-resolved if omitted)"},
                            "platforms": {"type": "array", "items": {"type": "string", "enum": ["tiktok", "instagram", "youtube"]}, "description": "Target platforms", "default": ["youtube"]},
                            "title": {"type": "string", "description": "Video title (defaults to the clip hook title)"},
                            "description": {"type": "string", "description": "Video description (defaults to the clip transcript)"},
                            "scheduled_date": {"type": "string", "description": "Optional ISO-8601 publish date (not yet applied)"}
                        },
                        "required": ["job_id"]
                    }
                },
                {
                    "name": "create_shorts_video",
                    "description": "Create an AI Shorts (text-to-video) task — NovaClip renders a vertical short with an AI actor, B-roll and music from a script.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "script": {"type": "string", "description": "The full script/narration for the short"},
                            "duration": {"type": "integer", "description": "Target duration in seconds", "default": 30},
                            "voice": {"type": "string", "description": "TTS voice ID", "default": "aura-asteria-en"},
                            "tts_provider": {"type": "string", "enum": ["elevenlabs", "deepgram-aura", "edge"], "default": "deepgram-aura"},
                            "bg_music": {"type": "string", "description": "Background music style", "default": "none"},
                            "subtitle_style": {"type": "string", "default": "bold"},
                            "cost_mode": {"type": "string", "enum": ["low", "premium"], "default": "low"},
                            "actor_description": {"type": "string", "description": "Description of the AI actor/presenter"},
                            "product_description": {"type": "string", "description": "Product/topic context for the short"},
                            "publish": {"type": "boolean", "description": "Auto-publish to YouTube after render via Upload-Post", "default": false},
                            "uploadpost_profile": {"type": "string", "description": "Upload-Post profile for auto-publish"},
                            "gemini_api_key": {"type": "string"},
                            "wavespeed_api_key": {"type": "string"},
                            "uploadpost_api_key": {"type": "string"}
                        },
                        "required": ["script"]
                    }
                },
                {
                    "name": "run_ai_edit",
                    "description": "Run an agentic NovaEdit instruction against a task's clips (trim, reframe, change captions, translate captions, boost originality, add meme, merge, delete).",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "job_id": {"type": "string", "description": "Task UUID"},
                            "instruction": {"type": "string", "description": "Natural-language edit instruction, e.g. 'trim the first clip'"},
                            "clip_ids": {"type": "array", "items": {"type": "string"}, "description": "Clip UUIDs to make available for editing (defaults to all clips)"},
                            "api_key": {"type": "string", "description": "Gemini API key"}
                        },
                        "required": ["job_id", "instruction"]
                    }
                },
                {
                    "name": "cancel_task",
                    "description": "Cancel a running task.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "job_id": {"type": "string", "description": "Task UUID"}
                        },
                        "required": ["job_id"]
                    }
                },
                {
                    "name": "resume_task",
                    "description": "Re-queue a cancelled or failed task.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "job_id": {"type": "string", "description": "Task UUID"}
                        },
                        "required": ["job_id"]
                    }
                },
                {
                    "name": "trim_clip",
                    "description": "Trim a clip by removing seconds from the start and/or end (re-encodes the clip).",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "job_id": {"type": "string", "description": "Task UUID"},
                            "clip_id": {"type": "string", "description": "Clip UUID (from list_clips)"},
                            "clip_index": {"type": "integer", "description": "0-based clip index (used when clip_id is omitted)", "default": 0},
                            "start_offset": {"type": "number", "description": "Seconds to remove from the start", "default": 0},
                            "end_offset": {"type": "number", "description": "Seconds to remove from the end", "default": 0}
                        },
                        "required": ["job_id"]
                    }
                },
                {
                    "name": "delete_clip",
                    "description": "Delete a clip from a task.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "job_id": {"type": "string", "description": "Task UUID"},
                            "clip_id": {"type": "string", "description": "Clip UUID (from list_clips)"},
                            "clip_index": {"type": "integer", "description": "0-based clip index (used when clip_id is omitted)", "default": 0}
                        },
                        "required": ["job_id"]
                    }
                },
                {
                    "name": "generate_titles",
                    "description": "Generate viral YouTube title ideas from a topic or transcript.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "topic": {"type": "string", "description": "Video topic"},
                            "transcript": {"type": "string", "description": "Video transcript (optional)"},
                            "tone": {"type": "string", "enum": ["viral", "educational", "story", "controversial", "listicle"], "default": "viral"},
                            "count": {"type": "integer", "default": 10},
                            "api_key": {"type": "string", "description": "Gemini API key"}
                        },
                        "required": []
                    }
                },
                {
                    "name": "generate_description",
                    "description": "Generate a YouTube video description with chapter timestamps from a topic or transcript.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "topic": {"type": "string", "description": "Video topic"},
                            "transcript": {"type": "string", "description": "Video transcript (optional)"},
                            "api_key": {"type": "string", "description": "Gemini API key"}
                        },
                        "required": []
                    }
                },
                {
                    "name": "generate_thumbnail",
                    "description": "Generate YouTube thumbnail image(s) with a Gemini image model. Returns base64 data URLs.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "title": {"type": "string", "description": "Video title"},
                            "style": {"type": "string", "enum": ["viral", "comparison", "text", "minimal", "step"], "default": "viral"},
                            "count": {"type": "integer", "description": "Number of variants (1-4)", "default": 1},
                            "extra_prompt": {"type": "string", "description": "Mandatory extra instructions"},
                            "video_context": {"type": "string", "description": "Transcript or summary of the video"},
                            "face_image_b64": {"type": "string", "description": "Base64 face image to feature on the thumbnail"},
                            "bg_image_b64": {"type": "string", "description": "Base64 background image"},
                            "image_model": {"type": "string", "description": "Gemini image model ID (default gemini-3.1-flash-image-preview; overrides llm_provider)"},
                            "llm_provider": {"type": "string", "description": "Deprecated alias for image_model — prefer image_model"},
                            "api_key": {"type": "string", "description": "Gemini API key"}
                        },
                        "required": ["title"]
                    }
                }
            ]
        })))),

        "tools/call" => {
            let params = req.params.unwrap_or(json!({}));
            let tool_name = params["name"].as_str().unwrap_or("");
            let empty_args = json!({});
            let args = params.get("arguments").unwrap_or(&empty_args);

            match tool_name {
                "process_video" => {
                    let url = args["url"].as_str().unwrap_or("");
                    if url.is_empty() {
                        return Ok(Json(mcp_tool_error(id, "url is required")));
                    }
                    let num_clips = args["num_clips"].as_i64().unwrap_or(5).clamp(1, 30) as i32;
                    let aspect_ratio = match args["aspect_ratio"].as_str().unwrap_or("9:16") {
                        "1:1" | "16:9" | "original" => args["aspect_ratio"].as_str().unwrap(),
                        _ => "9:16",
                    };
                    let add_subtitles = args["add_subtitles"].as_bool().unwrap_or(true);

                    let task_id = Uuid::new_v4();
                    let source_type = if url.contains("youtube.com") || url.contains("youtu.be") { "youtube" }
                        else if url.starts_with("upload://") { "upload" }
                        else { "video_url" };

                    sqlx::query(
                        r#"INSERT INTO tasks (id, source_url, source_type, aspect_ratio, num_clips,
                           font_family, font_size, font_color, caption_template, add_subtitles,
                           processing_mode, stt_provider, auto_vertical_reframe, reframe_preset)
                           VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)"#
                    )
                    .bind(task_id.to_string())
                    .bind(url)
                    .bind(source_type)
                    .bind(aspect_ratio)
                    .bind(num_clips)
                    .bind("THEBOLDFONT")
                    .bind(32)
                    .bind("#FFFFFF")
                    .bind("default")
                    .bind(if add_subtitles { 1 } else { 0 })
                    .bind("fast")
                    .bind("deepgram")
                    .bind(0)
                    .bind("talking_head")
                    .execute(&state.db)
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;

                    state.job_tx.send(task_id).await.ok();

                    Ok(Json(mcp_tool_result(id, json!({
                        "task_id": task_id.to_string(),
                        "status": "queued",
                        "message": format!("Task created: {} clips at {}", num_clips, aspect_ratio)
                    }))))
                },

                "get_job_status" => {
                    let job_id = args["job_id"].as_str().unwrap_or("");
                    if job_id.is_empty() {
                        return Ok(Json(mcp_tool_error(id, "job_id is required")));
                    }
                    let task = sqlx::query_as::<_, novaclip_db::Task>("SELECT * FROM tasks WHERE id = ?")
                        .bind(job_id)
                        .fetch_optional(&state.db)
                        .await
                        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;

                    match task {
                        Some(t) => Ok(Json(mcp_tool_result(id, json!({
                            "status": t.status,
                            "progress": t.progress,
                            "message": t.progress_message,
                            "created_at": t.created_at,
                            "completed_at": t.completed_at,
                            "error": t.error_message,
                        })))),
                        None => Ok(Json(mcp_tool_error(id, &format!("Task {} not found", job_id)))),
                    }
                },

                "list_clips" => {
                    let job_id = args["job_id"].as_str().unwrap_or("");
                    if job_id.is_empty() {
                        return Ok(Json(mcp_tool_error(id, "job_id is required")));
                    }
                    let clips = sqlx::query_as::<_, novaclip_db::GeneratedClip>(
                        "SELECT * FROM generated_clips WHERE task_id = ? ORDER BY clip_order ASC"
                    )
                    .bind(job_id)
                    .fetch_all(&state.db)
                    .await
                    .unwrap_or_default();

                    let clip_list: Vec<Value> = clips.iter().map(|c| json!({
                        "id": c.id,
                        "title": c.hook_title,
                        "duration": c.duration,
                        "start_time": c.start_time,
                        "end_time": c.end_time,
                        "hook_score": c.hook_score,
                        "virality_score": c.virality_score,
                        "file_path": c.file_path,
                    })).collect();

                    Ok(Json(mcp_tool_result(id, json!({
                        "clips": clip_list,
                        "count": clip_list.len(),
                    }))))
                },

                "get_quota" => {
                    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks")
                        .fetch_one(&state.db)
                        .await
                        .unwrap_or((0,));

                    Ok(Json(mcp_tool_result(id, json!({
                        "total_tasks": count.0,
                    }))))
                },

                "add_subtitles" => {
                    let job_id = args["job_id"].as_str().unwrap_or("");
                    if job_id.is_empty() {
                        return Ok(Json(mcp_tool_error(id, "job_id is required")));
                    }

                    let clips = sqlx::query_as::<_, novaclip_db::GeneratedClip>(
                        "SELECT * FROM generated_clips WHERE task_id = ? ORDER BY clip_order ASC"
                    )
                    .bind(job_id)
                    .fetch_all(&state.db)
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;

                    let clip = match args["clip_id"].as_str().filter(|s| !s.is_empty()) {
                        Some(cid) => clips.iter().find(|c| c.id == cid),
                        None => clips.get(args["clip_index"].as_i64().unwrap_or(0) as usize),
                    };
                    let Some(clip) = clip else {
                        return Ok(Json(mcp_tool_error(id, "Clip not found for this job")));
                    };

                    let src = std::path::PathBuf::from(&clip.file_path);
                    if !src.exists() {
                        return Ok(Json(mcp_tool_error(id, &format!("Clip file not found on disk: {}", clip.file_path))));
                    }

                    let transcript = clip.transcript_text.clone().unwrap_or_default();
                    if transcript.trim().is_empty() {
                        return Ok(Json(mcp_tool_error(id, "Clip has no transcript — cannot build subtitles")));
                    }

                    let font_size = args["font_size"].as_i64().unwrap_or(48) as i32;
                    let font_family = args["font_family"].as_str().unwrap_or("THEBOLDFONT").to_string();
                    let font_color = args["font_color"].as_str().unwrap_or("#FFFFFF");
                    let highlight_color = args["highlight_color"].as_str().unwrap_or("#FFFF00");
                    let uppercase = args["uppercase"].as_bool().unwrap_or(false);
                    let style = args["style"].as_str().unwrap_or("classic").to_string();
                    let position = args["position"].as_str().unwrap_or("bottom").to_string();

                    let primary_ass = if style == "karaoke" { hex_to_ass(highlight_color) } else { hex_to_ass(font_color) };

                    let duration = if clip.duration > 0.0 {
                        clip.duration
                    } else {
                        probe_video_duration(&clip.file_path).await.unwrap_or(15.0)
                    };
                    let (width, height) = probe_video_size(&clip.file_path).await.unwrap_or((1080, 1920));

                    let caption_text = if uppercase { transcript.to_uppercase() } else { transcript };
                    let words_per_line = if style == "karaoke" { 3 } else { 5 };
                    let srt = build_srt(&caption_text, duration, words_per_line);
                    let caption_count = srt.lines().filter(|l| l.contains("-->")).count();

                    let parent = src.parent().unwrap_or(std::path::Path::new("."));
                    let srt_path = parent.join(format!("subtitles_{}.srt", uuid::Uuid::new_v4()));
                    let out_path = parent.join(format!("captioned_{}.mp4", uuid::Uuid::new_v4()));
                    tokio::fs::write(&srt_path, &srt).await.ok();

                    let (align, margin_v) = match position.as_str() {
                        "top" => (8, 80),
                        "middle" => (5, 0),
                        _ => (2, 80),
                    };
                    let force_style = format!(
                        "Fontname='{}',Fontsize={},PrimaryColour={},OutlineColour=&H000000&,BorderStyle=1,Outline=3,Shadow=1,Bold=1,Alignment={},MarginV={}",
                        font_family.replace("'", "\\'"), font_size, primary_ass, align, margin_v
                    );
                    let srt_escaped = srt_path.to_str().unwrap()
                        .replace('\\', "/")
                        .replace(':', "\\:");

                    let status = tokio::process::Command::new("ffmpeg")
                        .args([
                            "-y", "-i", &clip.file_path,
                            "-vf", &format!("subtitles=filename='{}':force_style='{}'", srt_escaped, force_style),
                            "-c:v", "libx264", "-preset", "fast", "-crf", "18",
                            "-pix_fmt", "yuv420p", "-c:a", "copy",
                            "-movflags", "+faststart",
                            out_path.to_str().unwrap(),
                        ])
                        .status().await;

                    tokio::fs::remove_file(&srt_path).await.ok();

                    match status {
                        Ok(s) if s.success() => {
                            let new_path = out_path.to_string_lossy().to_string();
                            sqlx::query("UPDATE generated_clips SET file_path = ? WHERE id = ?")
                                .bind(&new_path)
                                .bind(&clip.id)
                                .execute(&state.db)
                                .await
                                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;
                            Ok(Json(mcp_tool_result(id, json!({
                                "clip_id": clip.id,
                                "file_path": new_path,
                                "captions": caption_count,
                                "style": style,
                                "position": position,
                                "width": width,
                                "height": height,
                            }))))
                        },
                        Ok(_) => {
                            tokio::fs::remove_file(&out_path).await.ok();
                            Ok(Json(mcp_tool_error(id, "FFmpeg subtitle burn failed")))
                        },
                        Err(e) => Ok(Json(mcp_tool_error(id, &format!("FFmpeg failed: {}", e)))),
                    }
                },

                "publish_clip" => {
                    let job_id = args["job_id"].as_str().unwrap_or("");
                    if job_id.is_empty() {
                        return Ok(Json(mcp_tool_error(id, "job_id is required")));
                    }

                    let api_key = args["api_key"].as_str().map(String::from)
                        .filter(|k| !k.is_empty())
                        .unwrap_or_else(|| std::env::var("UPLOADPOST_API_KEY").unwrap_or_default());
                    if api_key.is_empty() {
                        return Ok(Json(mcp_tool_error(id, "Missing Upload-Post API key (pass api_key or set UPLOADPOST_API_KEY)")));
                    }

                    let clips = sqlx::query_as::<_, novaclip_db::GeneratedClip>(
                        "SELECT * FROM generated_clips WHERE task_id = ? ORDER BY clip_order ASC"
                    )
                    .bind(job_id)
                    .fetch_all(&state.db)
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;

                    let clip = match args["clip_id"].as_str().filter(|s| !s.is_empty()) {
                        Some(cid) => clips.iter().find(|c| c.id == cid),
                        None => clips.get(args["clip_index"].as_i64().unwrap_or(0) as usize),
                    };
                    let Some(clip) = clip else {
                        return Ok(Json(mcp_tool_error(id, "Clip not found for this job")));
                    };

                    let src = std::path::PathBuf::from(&clip.file_path);
                    if !src.exists() {
                        return Ok(Json(mcp_tool_error(id, &format!("Clip file not found on disk: {}", clip.file_path))));
                    }

                    let platforms = args["platforms"].as_array()
                        .map(|a| a.iter().filter_map(|v| v.as_str()).filter(|p| *p == "youtube" || *p == "tiktok" || *p == "instagram").map(String::from).collect::<Vec<_>>())
                        .unwrap_or_else(|| vec!["youtube".to_string()]);
                    if platforms.is_empty() {
                        return Ok(Json(mcp_tool_error(id, "No supported platforms requested (youtube, tiktok, instagram)")));
                    }

                    let title = args["title"].as_str().map(String::from)
                        .filter(|t| !t.is_empty())
                        .or_else(|| clip.hook_title.clone())
                        .unwrap_or_else(|| "NovaClip Short".to_string());
                    let description = args["description"].as_str().map(String::from)
                        .filter(|d| !d.is_empty())
                        .or_else(|| clip.transcript_text.clone())
                        .unwrap_or_default();

                    let profile = match args["profile"].as_str().filter(|p| !p.is_empty()) {
                        Some(p) => p.to_string(),
                        None => match novaclip_worker::pipeline::uploadpost::resolve_profile(&api_key).await {
                            Ok(p) => p,
                            Err(e) => return Ok(Json(mcp_tool_error(id, &format!("Failed to resolve Upload-Post profile: {}", e)))),
                        },
                    };

                    match novaclip_worker::pipeline::uploadpost::publish_video(&api_key, &profile, &src, &title, &description, &platforms).await {
                        Ok(body) => Ok(Json(mcp_tool_result(id, json!({
                            "clip_id": clip.id,
                            "title": title,
                            "profile": profile,
                            "platforms": platforms,
                            "response": body,
                        })))),
                        Err(e) => Ok(Json(mcp_tool_error(id, &format!("Publish failed: {}", e)))),
                    }
                },

                "create_shorts_video" => {
                    let script = args["script"].as_str().unwrap_or("");
                    if script.trim().is_empty() {
                        return Ok(Json(mcp_tool_error(id, "script is required")));
                    }

                    let duration = args["duration"].as_i64().unwrap_or(30).clamp(5, 120) as i32;
                    let cost_mode = match args["cost_mode"].as_str().unwrap_or("low") {
                        "premium" => "premium",
                        _ => "low",
                    };
                    let tts_provider = args["tts_provider"].as_str().unwrap_or("deepgram-aura").to_string();
                    let voice = args["voice"].as_str().unwrap_or("aura-asteria-en").to_string();
                    let bg_music = args["bg_music"].as_str().unwrap_or("none").to_string();
                    let subtitle_style = args["subtitle_style"].as_str().unwrap_or("bold").to_string();
                    let actor_description = args["actor_description"].as_str().unwrap_or("").to_string();
                    let product_description = args["product_description"].as_str().unwrap_or("").to_string();
                    let publish = args["publish"].as_bool().unwrap_or(false);
                    let uploadpost_profile = args["uploadpost_profile"].as_str().unwrap_or("").to_string();

                    let gemini_key = args["gemini_api_key"].as_str().map(String::from).filter(|k| !k.is_empty())
                        .or_else(|| std::env::var("GEMINI_API_KEY").ok().filter(|k| !k.is_empty()))
                        .unwrap_or_default();
                    let wavespeed_key = args["wavespeed_api_key"].as_str().map(String::from).filter(|k| !k.is_empty())
                        .or_else(|| std::env::var("WAVESPEED_API_KEY").ok().filter(|k| !k.is_empty()))
                        .unwrap_or_default();
                    let uploadpost_key = args["uploadpost_api_key"].as_str().map(String::from).filter(|k| !k.is_empty())
                        .or_else(|| std::env::var("UPLOADPOST_API_KEY").ok().filter(|k| !k.is_empty()))
                        .unwrap_or_default();
                    let llm_provider = args["llm_provider"].as_str().filter(|m| !m.is_empty())
                        .unwrap_or("gemini-3.1-flash-lite").to_string();

                    let studio_payload = json!({
                        "script": script.trim(),
                        "mode": "ai-shorts",
                        "aspect_ratio": "9:16",
                        "llm_provider": llm_provider,
                        "tts_provider": tts_provider,
                        "voice": voice,
                        "duration": duration,
                        "subtitle_style": subtitle_style,
                        "bg_music": bg_music,
                        "shorts_payload": {
                            "cost_mode": cost_mode,
                            "actor_source": "mcp",
                            "actor_description": actor_description,
                            "product_description": product_description,
                            "publish": publish,
                            "uploadpost_profile": uploadpost_profile,
                        },
                        "api_keys": {
                            "gemini_key": gemini_key,
                            "deepgram_key": "",
                            "openrouter_key": "",
                            "elevenlabs_key": "",
                            "pexels_key": "",
                            "pixabay_key": "",
                            "wavespeed_key": wavespeed_key,
                            "uploadpost_key": uploadpost_key,
                        }
                    });

                    let task_id = Uuid::new_v4();
                    let payload_json = serde_json::to_string(&studio_payload).unwrap_or_default();

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
                    .bind("studio://ai-shorts")
                    .bind("AI Shorts")
                    .bind("studio")
                    .bind("9:16")
                    .bind(1i32)
                    .bind("THEBOLDFONT")
                    .bind(28i32)
                    .bind("#FFFFFF")
                    .bind("default")
                    .bind(1i32)
                    .bind(0i32)
                    .bind("fast")
                    .bind(0i32)
                    .bind(900i32)
                    .bind(0i32)
                    .bind("[]")
                    .bind(&gemini_key)
                    .bind("")
                    .bind("")
                    .bind("deepgram")
                    .bind(0i32)
                    .bind("talking_head")
                    .bind(1i32)
                    .bind("single")
                    .bind(0i32)
                    .bind(0i32)
                    .bind("none")
                    .bind("")
                    .bind(Option::<String>::None)
                    .bind(Some(payload_json))
                    .bind("#22d3ee")
                    .bind("word_pop")
                    .bind(1i32)
                    .bind("top_right")
                    .bind(0.8f64)
                    .bind(Option::<String>::None)
                    .bind(&llm_provider)
                    .bind(Option::<String>::None)
                    .execute(&state.db)
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;

                    state.job_tx.send(task_id).await.ok();

                    Ok(Json(mcp_tool_result(id, json!({
                        "task_id": task_id.to_string(),
                        "status": "queued",
                        "message": "AI Shorts task created and queued"
                    }))))
                },

                "run_ai_edit" => {
                    let job_id = args["job_id"].as_str().unwrap_or("");
                    if job_id.is_empty() {
                        return Ok(Json(mcp_tool_error(id, "job_id is required")));
                    }
                    let instruction = args["instruction"].as_str().unwrap_or("");
                    if instruction.is_empty() {
                        return Ok(Json(mcp_tool_error(id, "instruction is required")));
                    }
                    let task_id = match Uuid::parse_str(job_id) {
                        Ok(u) => u,
                        Err(_) => return Ok(Json(mcp_tool_error(id, "job_id must be a valid UUID"))),
                    };

                    let clip_ids: Vec<Uuid> = match args["clip_ids"] {
                        Value::Array(ref arr) => arr.iter()
                            .filter_map(|v| v.as_str())
                            .filter_map(|s| Uuid::parse_str(s).ok())
                            .collect(),
                        _ => {
                            let clips = sqlx::query_as::<_, novaclip_db::GeneratedClip>(
                                "SELECT * FROM generated_clips WHERE task_id = ? ORDER BY clip_order ASC"
                            )
                            .bind(job_id)
                            .fetch_all(&state.db)
                            .await
                            .unwrap_or_default();
                            clips.iter().filter_map(|c| Uuid::parse_str(&c.id).ok()).collect()
                        }
                    };
                    if clip_ids.is_empty() {
                        return Ok(Json(mcp_tool_error(id, "No clips found for this job")));
                    }

                    let api_key = args["api_key"].as_str().map(String::from);
                    match crate::routes::ai_edit::execute_ai_edit(&state, &task_id, clip_ids, instruction.to_string(), api_key).await {
                        Ok(Json(v)) => Ok(Json(mcp_tool_result(id, v))),
                        Err((_, Json(e))) => Ok(Json(mcp_tool_error(id, e["error"].as_str().unwrap_or("AI edit failed")))),
                    }
                },

                "cancel_task" => {
                    let job_id = args["job_id"].as_str().unwrap_or("");
                    if job_id.is_empty() {
                        return Ok(Json(mcp_tool_error(id, "job_id is required")));
                    }
                    sqlx::query(
                        "UPDATE tasks SET cancel_requested = 1, status = 'cancelled', progress_message = 'Cancelled via MCP', updated_at = datetime('now') WHERE id = ?"
                    )
                    .bind(job_id)
                    .execute(&state.db)
                    .await
                    .ok();
                    Ok(Json(mcp_tool_result(id, json!({
                        "job_id": job_id,
                        "status": "cancelled"
                    }))))
                },

                "resume_task" => {
                    let job_id = args["job_id"].as_str().unwrap_or("");
                    if job_id.is_empty() {
                        return Ok(Json(mcp_tool_error(id, "job_id is required")));
                    }
                    let task = sqlx::query_as::<_, novaclip_db::Task>("SELECT * FROM tasks WHERE id = ?")
                        .bind(job_id)
                        .fetch_optional(&state.db)
                        .await
                        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;
                    if task.is_none() {
                        return Ok(Json(mcp_tool_error(id, &format!("Task {} not found", job_id))));
                    }
                    sqlx::query(
                        "UPDATE tasks SET status = 'queued', cancel_requested = 0, progress = 0, progress_message = 'Re-queued via MCP', updated_at = datetime('now') WHERE id = ?"
                    )
                    .bind(job_id)
                    .execute(&state.db)
                    .await
                    .ok();
                    if let Ok(parsed) = Uuid::parse_str(job_id) {
                        state.job_tx.send(parsed).await.ok();
                    }
                    Ok(Json(mcp_tool_result(id, json!({
                        "job_id": job_id,
                        "status": "queued"
                    }))))
                },

                "trim_clip" => {
                    let job_id = args["job_id"].as_str().unwrap_or("");
                    if job_id.is_empty() {
                        return Ok(Json(mcp_tool_error(id, "job_id is required")));
                    }
                    let clip = match find_clip_for_args(&state, job_id, args).await {
                        Ok(c) => c,
                        Err(e) => return Ok(Json(mcp_tool_error(id, &e))),
                    };
                    let src = std::path::PathBuf::from(&clip.file_path);
                    if !src.exists() {
                        return Ok(Json(mcp_tool_error(id, &format!("Clip file not found on disk: {}", clip.file_path))));
                    }

                    let start_offset = args["start_offset"].as_f64().unwrap_or(0.0).max(0.0);
                    let end_offset = args["end_offset"].as_f64().unwrap_or(0.0).max(0.0);
                    let duration = clip.duration - start_offset - end_offset;
                    if duration <= 0.0 {
                        return Ok(Json(mcp_tool_error(id, "Trim would produce empty clip")));
                    }

                    let parent = src.parent().unwrap_or(std::path::Path::new("."));
                    let output_name = format!("trim_{}.mp4", Uuid::new_v4().to_string().replace("-", ""));
                    let output_path = parent.join(&output_name);

                    let status = tokio::process::Command::new("ffmpeg")
                        .args([
                            "-y", "-ss", &start_offset.to_string(), "-i", &clip.file_path,
                            "-t", &duration.to_string(), "-c:v", "libx264", "-crf", "18",
                            "-preset", "fast", "-c:a", "aac",
                            output_path.to_str().unwrap(),
                        ])
                        .status().await;

                    match status {
                        Ok(s) if s.success() => {
                            sqlx::query(
                                "UPDATE generated_clips SET file_path = ?, filename = ?, duration = ?, updated_at = datetime('now') WHERE id = ?"
                            )
                            .bind(output_path.to_str().unwrap())
                            .bind(&output_name)
                            .bind(duration)
                            .bind(&clip.id)
                            .execute(&state.db)
                            .await
                            .ok();
                            Ok(Json(mcp_tool_result(id, json!({
                                "clip_id": clip.id,
                                "duration": duration,
                                "file_path": output_path.to_string_lossy(),
                            }))))
                        },
                        Ok(_) => Ok(Json(mcp_tool_error(id, "FFmpeg trim failed"))),
                        Err(e) => Ok(Json(mcp_tool_error(id, &format!("FFmpeg failed: {}", e)))),
                    }
                },

                "delete_clip" => {
                    let job_id = args["job_id"].as_str().unwrap_or("");
                    if job_id.is_empty() {
                        return Ok(Json(mcp_tool_error(id, "job_id is required")));
                    }
                    let clip = match find_clip_for_args(&state, job_id, args).await {
                        Ok(c) => c,
                        Err(e) => return Ok(Json(mcp_tool_error(id, &e))),
                    };
                    sqlx::query("DELETE FROM generated_clips WHERE id = ? AND task_id = ?")
                        .bind(&clip.id)
                        .bind(job_id)
                        .execute(&state.db)
                        .await
                        .ok();
                    Ok(Json(mcp_tool_result(id, json!({
                        "clip_id": clip.id,
                        "status": "deleted"
                    }))))
                },

                "generate_titles" => {
                    let topic = args["topic"].as_str().unwrap_or("");
                    let transcript = args["transcript"].as_str().unwrap_or("");
                    if topic.trim().is_empty() && transcript.trim().is_empty() {
                        return Ok(Json(mcp_tool_error(id, "topic or transcript is required")));
                    }
                    let api_key = match mcp_gemini_key(args) {
                        Ok(k) => k,
                        Err(e) => return Ok(Json(mcp_tool_error(id, &e))),
                    };
                    let model = mcp_text_model(args);

                    let tone_desc = match args["tone"].as_str().unwrap_or("viral") {
                        "educational" => "educational how-to style",
                        "story" => "story-driven curiosity gap style",
                        "controversial" => "controversial debate-sparking style",
                        "listicle" => "listicle top-N style",
                        _ => "viral clickbaity style",
                    };
                    let count = args["count"].as_i64().unwrap_or(10).clamp(5, 20) as usize;

                    let mut context = String::new();
                    if !transcript.trim().is_empty() {
                        context.push_str(&format!("TRANSCRIPT:\n{}\n", transcript));
                    }
                    if !topic.trim().is_empty() {
                        context.push_str(&format!("VIDEO TOPIC: {}\n", topic));
                    }

                    let prompt = format!(
                        "You are a YouTube title expert who creates viral, click-worthy titles.\n\n\
                         Analyze the video and its transcript, then suggest {} YouTube titles that would maximize CTR.\n\n\
                         {}\n\
                         RULES:\n\
                         - Titles must be under 70 characters\n\
                         - Use power words, curiosity gaps, and emotional triggers\n\
                         - Mix styles: how-to, listicle, story-driven, controversial, question-based ({})\n\
                         - Make them specific to the actual content, not generic\n\
                         - Titles should be in the SAME LANGUAGE as the video\n\n\
                         Also provide a brief summary of the video content (2-3 sentences), and pick the TOP 2 most recommended titles with concise reasons (0-based index).\n\n\
                         OUTPUT JSON: {{\"titles\": [\"title1\", ...], \"transcript_summary\": \"Brief summary...\", \"language\": \"...\", \"recommended\": [{{\"index\": 0, \"reason\": \"...\"}}]}}",
                        count, context, tone_desc
                    );

                    let text = gemini_text(&api_key, &model, "You are a YouTube title optimization expert. Return ONLY valid JSON.", &prompt, 4096, 0.8).await
                        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(json!({"error": e}))))?;
                    let mut result = parse_json_block(&text)
                        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(json!({"error": format!("Parse error: {}", e)}))))?;
                    if let Some(titles) = result.get_mut("titles").and_then(|t| t.as_array_mut()) {
                        titles.truncate(count);
                    }
                    Ok(Json(mcp_tool_result(id, result)))
                },

                "generate_description" => {
                    let topic = args["topic"].as_str().unwrap_or("");
                    let transcript = args["transcript"].as_str().unwrap_or("");
                    if topic.trim().is_empty() && transcript.trim().is_empty() {
                        return Ok(Json(mcp_tool_error(id, "topic or transcript is required")));
                    }
                    let api_key = match mcp_gemini_key(args) {
                        Ok(k) => k,
                        Err(e) => return Ok(Json(mcp_tool_error(id, &e))),
                    };
                    let model = mcp_text_model(args);

                    let mut context = String::new();
                    if !transcript.trim().is_empty() {
                        context.push_str(&format!("TRANSCRIPT:\n{}\n", transcript));
                    }
                    if !topic.trim().is_empty() {
                        context.push_str(&format!("VIDEO TOPIC: {}\n", topic));
                    }

                    let prompt = format!(
                        "You are a YouTube SEO expert. Generate a complete YouTube video description with chapter markers.\n\n\
                         VIDEO REFERENCE:\n{}\n\
                         REQUIREMENTS:\n\
                         1. Start with a compelling 2-3 sentence summary hook\n\
                         2. Add relevant CTAs (subscribe, like, comment)\n\
                         3. Generate YouTube CHAPTERS: first chapter MUST start at 0:00, minimum 3 chapters, each at least 10 seconds apart, concise descriptive titles\n\
                         4. Add 5-10 relevant hashtags at the end\n\
                         5. Keep the total description under 5000 characters\n\
                         6. Write the description in the SAME LANGUAGE as the video\n\n\
                         OUTPUT JSON: {{\"description\": \"full description text with formatting and chapters\", \"chapters\": [{{\"time\": \"0:00\", \"title\": \"Introduction\"}}]}}",
                        context
                    );

                    let text = gemini_text(&api_key, &model, "You are a YouTube SEO and description expert. Return ONLY valid JSON.", &prompt, 4096, 0.6).await
                        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(json!({"error": e}))))?;
                    let result = parse_json_block(&text)
                        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(json!({"error": format!("Parse error: {}", e)}))))?;
                    Ok(Json(mcp_tool_result(id, result)))
                },

                "generate_thumbnail" => {
                    let title = args["title"].as_str().unwrap_or("");
                    if title.trim().is_empty() {
                        return Ok(Json(mcp_tool_error(id, "title is required")));
                    }
                    let api_key = match mcp_gemini_key(args) {
                        Ok(k) => k,
                        Err(e) => return Ok(Json(mcp_tool_error(id, &e))),
                    };
                    let image_model = args["image_model"].as_str()
                        .filter(|m| !m.is_empty())
                        .or_else(|| args["llm_provider"].as_str().filter(|m| !m.is_empty()))
                        .map(String::from)
                        .unwrap_or_else(|| {
                            std::env::var("GEMINI_THUMBNAIL_MODEL").unwrap_or_else(|_| "gemini-3.1-flash-image-preview".into())
                        });

                    let count = args["count"].as_i64().unwrap_or(1).clamp(1, 4) as usize;
                    let style = args["style"].as_str().unwrap_or("viral");
                    let video_context = args["video_context"].as_str().unwrap_or("");
                    let extra_prompt = args["extra_prompt"].as_str().unwrap_or("");
                    let face_b64 = args["face_image_b64"].as_str().unwrap_or("");
                    let bg_b64 = args["bg_image_b64"].as_str().unwrap_or("");

                    let style_prompt = match style {
                        "comparison" => "before/after split comparison thumbnail style",
                        "text" => "bold text overlay with high contrast",
                        "minimal" => "clean minimal design with one focal element",
                        "step" => "numbered step-by-step layout",
                        _ => "viral YouTube reaction-style thumbnail with face and bold text",
                    };

                    let mut prompt = format!(
                        "Generate a professional, eye-catching YouTube thumbnail image.\n\n\
                         VIDEO TITLE (for reference — do NOT put the full title on the thumbnail): \"{}\"\n\n",
                        title
                    );
                    if !video_context.trim().is_empty() {
                        prompt.push_str(&format!("VIDEO CONTEXT:\n{}\n\n", video_context));
                    }
                    prompt.push_str(
                        "TEXT ON THE THUMBNAIL:\n- Create a SHORT visual hook: 1 to 5 words maximum\n- It should COMPLEMENT the title, not repeat it\n- Use ALL CAPS, split into 2-3 lines\n",
                    );
                    if !extra_prompt.trim().is_empty() {
                        prompt.push_str(&format!("MANDATORY USER INSTRUCTIONS (override default behavior):\n{}\n\n", extra_prompt));
                    }
                    prompt.push_str(&format!(
                        "DESIGN REQUIREMENTS:\n- {}\n- Large bold high-contrast text, vibrant colors, clean composition, no clutter, no watermark, exactly 16:9.",
                        style_prompt
                    ));
                    if !face_b64.is_empty() {
                        prompt.push_str("\n- Include the provided face/person prominently with an exaggerated expression (surprise, excitement, shock)");
                    }
                    if !bg_b64.is_empty() {
                        prompt.push_str("\n- Use the provided background image as the base/backdrop");
                    }

                    let client = reqwest::Client::new();
                    let mut thumbnails: Vec<Value> = Vec::new();

                    for i in 0..count {
                        let mut parts: Vec<Value> = Vec::new();
                        if !face_b64.is_empty() {
                            parts.push(json!({"inlineData": {"mimeType": "image/png", "data": face_b64}}));
                        }
                        if !bg_b64.is_empty() {
                            parts.push(json!({"inlineData": {"mimeType": "image/png", "data": bg_b64}}));
                        }
                        parts.push(json!({"text": prompt}));

                        let body = json!({
                            "systemInstruction": {"parts": [{"text": "You are a YouTube thumbnail designer. Return the image AND a short JSON block with the design rationale."}]},
                            "contents": [{"role": "user", "parts": parts}],
                            "generationConfig": {
                                "temperature": 0.7,
                                "maxOutputTokens": 1024,
                                "responseModalities": ["TEXT", "IMAGE"],
                                "imageConfig": {"aspectRatio": "16:9", "imageSize": "2K"}
                            }
                        });
                        let url = format!(
                            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                            image_model, api_key
                        );
                        let resp = client.post(&url).json(&body)
                            .timeout(Duration::from_secs(90))
                            .send().await
                            .map_err(|e| (StatusCode::BAD_GATEWAY, Json(json!({"error": format!("Gemini error: {}", e)}))))?;

                        let response_parts = resp.json::<Value>().await
                            .ok().and_then(|r| r.pointer("/candidates/0/content/parts").and_then(|v| v.as_array()).cloned())
                            .ok_or_else(|| (StatusCode::BAD_GATEWAY, Json(json!({"error": "Empty Gemini response"}))))?;

                        let mut text_out = String::new();
                        let mut image_b64 = String::new();
                        let mut mime = "image/png".to_string();
                        for part in response_parts {
                            if let Some(t) = part.get("text").and_then(|v| v.as_str()) {
                                text_out = t.to_string();
                            }
                            if let Some(inline) = part.get("inlineData") {
                                if let Some(d) = inline.get("data").and_then(|v| v.as_str()) {
                                    image_b64 = d.to_string();
                                }
                                if let Some(m) = inline.get("mimeType").and_then(|v| v.as_str()) {
                                    mime = m.to_string();
                                }
                            }
                        }
                        if image_b64.is_empty() {
                            return Ok(Json(mcp_tool_error(id, &format!("Gemini returned no image for variant {} — the model '{}' cannot generate images", i + 1, image_model))));
                        }
                        let doc = parse_json_block(&text_out).unwrap_or_else(|_| json!({"prompt": text_out}));
                        thumbnails.push(json!({
                            "index": i,
                            "image_url": format!("data:{};base64,{}", mime, image_b64),
                            "mime_type": mime,
                            "prompt": doc.get("prompt").and_then(|v| v.as_str()).unwrap_or(""),
                        }));
                    }

                    Ok(Json(mcp_tool_result(id, json!({
                        "thumbnails": thumbnails,
                        "count": thumbnails.len(),
                        "image_url": thumbnails.first().and_then(|t| t.get("image_url")).cloned().unwrap_or(Value::Null),
                    }))))
                },

                _ => Ok(Json(mcp_tool_error(id, &format!("Unknown tool: {}", tool_name)))),
            }
        },

        _ => Ok(Json(json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32601,
                "message": format!("Method not found: {}", req.method)
            }
        }))),
    }
}

fn mcp_response(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}

fn mcp_tool_result(id: Value, content: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "content": [{
                "type": "text",
                "text": content.to_string()
            }]
        }
    })
}

fn mcp_tool_error(id: Value, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "content": [{
                "type": "text",
                "text": format!("Error: {}", message)
            }],
            "isError": true
        }
    })
}

/// Convert hex #RRGGBB to ASS &H00BBGGRR& format
fn hex_to_ass(hex: &str) -> String {
    let h = hex.trim_start_matches('#');
    if h.len() != 6 { return "&H00FFFFFF&".to_string(); }
    format!("&H00{}{}{}&", &h[4..6], &h[2..4], &h[0..2])
}

fn srt_timestamp(secs: f64) -> String {
    let secs = secs.max(0.0);
    let h = (secs / 3600.0) as u32;
    let m = ((secs % 3600.0) / 60.0) as u32;
    let s = (secs % 60.0) as u32;
    let ms = (secs.fract() * 1000.0) as u32;
    format!("{:02}:{:02}:{:02},{:03}", h, m, s, ms)
}

/// Split a transcript into caption lines timed evenly across the clip duration.
fn build_srt(transcript: &str, duration: f64, words_per_line: usize) -> String {
    let words: Vec<&str> = transcript.split_whitespace().collect();
    if words.is_empty() { return String::new(); }

    let lines: Vec<String> = words.chunks(words_per_line.max(1))
        .map(|chunk| chunk.join(" "))
        .collect();

    let n = lines.len() as f64;
    let start = (0.4_f64).min(duration * 0.1);
    let end = (duration - 0.4).max(start + 0.1);
    let span = (end - start) / n;

    let mut srt = String::new();
    for (i, line) in lines.iter().enumerate() {
        let t0 = start + (i as f64) * span;
        let t1 = if i == lines.len() - 1 { end } else { t0 + span };
        srt.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            i + 1,
            srt_timestamp(t0),
            srt_timestamp(t1),
            line
        ));
    }
    srt
}

async fn probe_video_duration(path: &str) -> Option<f64> {
    let out = tokio::process::Command::new("ffprobe")
        .args([
            "-v", "error",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
            path,
        ])
        .output().await.ok()?;
    String::from_utf8_lossy(&out.stdout).trim().parse().ok()
}

async fn probe_video_size(path: &str) -> Option<(u32, u32)> {
    let out = tokio::process::Command::new("ffprobe")
        .args([
            "-v", "error",
            "-select_streams", "v:0",
            "-show_entries", "stream=width,height",
            "-of", "csv=p=0:s=x",
            path,
        ])
        .output().await.ok()?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if let Some((w, h)) = stdout.split_once('x') {
        let w: u32 = w.trim().parse().ok()?;
        let h: u32 = h.trim().parse().ok()?;
        Some((w, h))
    } else {
        None
    }
}

/// Resolve a clip for a job by `clip_id` (preferred) or 0-based `clip_index`.
async fn find_clip_for_args(
    state: &AppState,
    job_id: &str,
    args: &Value,
) -> Result<novaclip_db::GeneratedClip, String> {
    let clips = sqlx::query_as::<_, novaclip_db::GeneratedClip>(
        "SELECT * FROM generated_clips WHERE task_id = ? ORDER BY clip_order ASC"
    )
    .bind(job_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| format!("DB error: {}", e))?;

    match args.get("clip_id").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
        Some(cid) => clips.into_iter().find(|c| c.id == cid),
        None => clips.into_iter().nth(args.get("clip_index").and_then(|v| v.as_i64()).unwrap_or(0) as usize),
    }
    .ok_or_else(|| "Clip not found for this job".to_string())
}

/// Resolve a Gemini API key from `api_key` arg or GEMINI_API_KEY env.
fn mcp_gemini_key(args: &Value) -> Result<String, String> {
    args.get("api_key")
        .and_then(|v| v.as_str())
        .map(String::from)
        .filter(|k| !k.is_empty())
        .or_else(|| std::env::var("GEMINI_API_KEY").ok().filter(|k| !k.is_empty()))
        .ok_or_else(|| "Missing Gemini API key (pass api_key or set GEMINI_API_KEY)".to_string())
}

/// Resolve the text model from `llm_provider` arg, GEMINI_MODEL env, or default.
fn mcp_text_model(args: &Value) -> String {
    args.get("llm_provider")
        .and_then(|v| v.as_str())
        .filter(|m| !m.is_empty())
        .map(String::from)
        .unwrap_or_else(|| std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-3.1-flash-lite".to_string()))
}

/// Run a single Gemini text generation request, returning the raw JSON-ish text.
async fn gemini_text(
    api_key: &str,
    model: &str,
    system: &str,
    prompt: &str,
    max_tokens: u32,
    temperature: f64,
) -> Result<String, String> {
    let body = json!({
        "systemInstruction": {"parts": [{"text": system}]},
        "contents": [{"role": "user", "parts": [{"text": prompt}]}],
        "generationConfig": {
            "responseMimeType": "application/json",
            "maxOutputTokens": max_tokens,
            "temperature": temperature
        }
    });
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );
    let resp = reqwest::Client::new()
        .post(&url)
        .json(&body)
        .timeout(Duration::from_secs(120))
        .send()
        .await
        .map_err(|e| format!("Gemini request failed: {}", e))?;

    let status = resp.status();
    let parsed = resp.json::<Value>().await
        .map_err(|e| format!("Gemini response parse failed: {}", e))?;

    if !status.is_success() {
        let msg = parsed.pointer("/error/message")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown Gemini error");
        return Err(format!("Gemini API error ({}): {}", status, msg));
    }

    parsed.pointer("/candidates/0/content/parts/0/text")
        .and_then(|v| v.as_str())
        .map(|t| t.to_string())
        .ok_or_else(|| "Gemini returned no text output".to_string())
}
