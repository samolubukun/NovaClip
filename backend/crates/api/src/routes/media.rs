use axum::{extract::State, routing::{get, post}, Router, Json};
use axum::http::StatusCode;
use serde_json::{json, Value};
use crate::state::AppState;

pub fn media_router() -> Router<AppState> {
    Router::new()
        .route("/media/caption-templates", get(get_caption_templates))
        .route("/media/fonts", get(get_fonts))
        .route("/media/broll/status", get(broll_status))
        .route("/media/upload", post(upload_video))
}

async fn get_caption_templates() -> Json<Value> {
    Json(json!({
        "templates": [
            {"id":"default","name":"Default","description":"Punchy word-by-word captions with pop highlight and emojis","animation":"karaoke","highlight_color":"#FFE000"},
            {"id":"hormozi","name":"Hormozi","description":"Bold green highlight with pill behind active word","animation":"karaoke","highlight_color":"#00FF66"},
            {"id":"mrbeast","name":"MrBeast","description":"Large yellow text with red pop highlights","animation":"karaoke","highlight_color":"#FF2D2D"},
            {"id":"minimal","name":"Minimal","description":"Clean, subtle captions with soft background","animation":"fade","highlight_color":"#FFFFFF"},
            {"id":"tiktok","name":"TikTok","description":"TikTok-style with pink pop highlights","animation":"karaoke","highlight_color":"#FE2C55"},
            {"id":"neon","name":"Neon","description":"Glowing neon text with magenta highlights","animation":"karaoke","highlight_color":"#FF00FF"},
            {"id":"podcast","name":"Podcast","description":"Professional podcast-style captions","animation":"karaoke","highlight_color":"#FFB800"}
        ]
    }))
}

async fn get_fonts(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "fonts": [
            {"name":"THEBOLDFONT","display_name":"The Bold Font","format":"ttf"},
            {"name":"TikTokSans-Regular","display_name":"TikTok Sans Regular","format":"ttf"},
            {"name":"TikTokSans-Bold","display_name":"TikTok Sans Bold","format":"ttf"}
        ]
    }))
}

async fn broll_status(State(state): State<AppState>) -> Json<Value> {
    let configured = state.config.pexels_api_key.is_some();
    Json(json!({
        "configured": configured,
        "provider": if configured { Some("pexels") } else { None::<&str> }
    }))
}

async fn upload_video(
    State(state): State<AppState>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let uploads_dir = format!("{}/uploads", state.config.temp_dir);
    tokio::fs::create_dir_all(&uploads_dir).await.ok();

    while let Some(field) = multipart.next_field().await
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))))? {
        let name = field.name().unwrap_or("").to_string();
        if name == "video" {
            let filename = field.file_name().unwrap_or("upload.mp4").to_string();
            let ext = std::path::Path::new(&filename)
                .extension().and_then(|e| e.to_str()).unwrap_or("mp4");
            let unique_name = format!("{}.{}", uuid::Uuid::new_v4(), ext);
            let path = format!("{}/{}", uploads_dir, unique_name);

            let data = field.bytes().await
                .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))))?;

            if data.len() as u64 > state.config.max_video_upload_bytes {
                return Err((StatusCode::PAYLOAD_TOO_LARGE, Json(json!({"error": "File too large"}))));
            }

            tokio::fs::write(&path, &data).await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;

            return Ok(Json(json!({
                "message": "Video uploaded",
                "video_path": format!("upload://{}", unique_name)
            })));
        }
    }
    Err((StatusCode::BAD_REQUEST, Json(json!({"error": "No video field in form"}))))
}
