use axum::{extract::{Path, State}, routing::get, Router, Json};
use axum::http::{HeaderValue, StatusCode};
use serde_json::{json, Value};
use uuid::Uuid;
use crate::state::AppState;

pub fn clips_router() -> Router<AppState> {
    Router::new()
        .route("/tasks/{id}/clips/{clip_id}/file", get(serve_clip))
        .route("/tasks/{id}/clips/{clip_id}/export", get(export_clip))
}

async fn serve_clip(
    State(state): State<AppState>,
    Path((task_id, clip_id)): Path<(Uuid, Uuid)>,
) -> Result<impl axum::response::IntoResponse, (StatusCode, Json<Value>)> {
    let clip = sqlx::query_as::<_, novaclip_db::GeneratedClip>(
        "SELECT * FROM generated_clips WHERE id = ? AND task_id = ?"
    )
    .bind(clip_id.to_string())
    .bind(task_id.to_string())
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?
    .ok_or_else(|| (StatusCode::NOT_FOUND, Json(json!({"error": "Clip not found"}))))?;

    let path = std::path::Path::new(&clip.file_path);
    if !path.exists() {
        return Err((StatusCode::NOT_FOUND, Json(json!({"error": "Clip file not found on disk"}))));
    }

    let file = tokio::fs::File::open(path).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;
    let stream = tokio_util::io::ReaderStream::new(file);
    let body = axum::body::Body::from_stream(stream);

    // Use owned HeaderValue to avoid returning references to temporaries
    let content_disp = HeaderValue::try_from(
        format!("inline; filename=\"{}\"", clip.filename)
    ).unwrap_or_else(|_| HeaderValue::from_static("inline"));

    Ok((
        [
            (axum::http::header::CONTENT_TYPE, HeaderValue::from_static("video/mp4")),
            (axum::http::header::CONTENT_DISPOSITION, content_disp),
        ],
        body,
    ))
}

async fn export_clip(
    State(state): State<AppState>,
    Path((task_id, clip_id)): Path<(Uuid, Uuid)>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl axum::response::IntoResponse, (StatusCode, Json<Value>)> {
    let preset = params.get("preset").map(|s| s.as_str()).unwrap_or("tiktok");
    let (width, height, bitrate) = match preset {
        "reels" => (1080, 1920, "12M"),
        "shorts" => (1080, 1920, "10M"),
        _ => (1080, 1920, "10M"),
    };

    let clip = sqlx::query_as::<_, novaclip_db::GeneratedClip>(
        "SELECT * FROM generated_clips WHERE id = ? AND task_id = ?"
    )
    .bind(clip_id.to_string())
    .bind(task_id.to_string())
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?
    .ok_or_else(|| (StatusCode::NOT_FOUND, Json(json!({"error": "Clip not found"}))))?;

    // Cross-platform temp path (fixes the hardcoded /tmp/ bug)
    let temp_output = std::env::temp_dir()
        .join(format!("novaclip_export_{}.mp4", Uuid::new_v4()));
    let temp_output_str = temp_output.to_str().unwrap().to_string();

    let scale = format!(
        "scale={}:{}:force_original_aspect_ratio=decrease:flags=lanczos,pad={}:{}:(ow-iw)/2:(oh-ih)/2,setsar=1",
        width, height, width, height
    );
    let status = tokio::process::Command::new("ffmpeg")
        .args(["-y", "-i", &clip.file_path, "-vf", &scale,
               "-c:v", "libx264", "-preset", "slow", "-crf", "18",
               "-maxrate", bitrate, "-bufsize", "20M",
               "-pix_fmt", "yuv420p", "-c:a", "aac", "-b:a", "192k",
               "-movflags", "+faststart", &temp_output_str])
        .status().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;

    if !status.success() {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Export failed"}))));
    }

    let file = tokio::fs::File::open(&temp_output).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;
    let stream = tokio_util::io::ReaderStream::new(file);
    let body = axum::body::Body::from_stream(stream);

    let dl_name = format!("{}_{}.mp4", clip.filename.replace(".mp4", ""), preset);
    let content_disp = HeaderValue::try_from(
        format!("attachment; filename=\"{}\"", dl_name)
    ).unwrap_or_else(|_| HeaderValue::from_static("attachment"));

    Ok((
        [
            (axum::http::header::CONTENT_TYPE, HeaderValue::from_static("video/mp4")),
            (axum::http::header::CONTENT_DISPOSITION, content_disp),
        ],
        body,
    ))
}
