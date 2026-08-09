use std::collections::HashMap;
use std::time::Duration;

use axum::{
    extract::{DefaultBodyLimit, Multipart, State},
    http::StatusCode,
    routing::post,
    Json, Router,
};
use base64::Engine;
use serde::Deserialize;
use serde_json::{json, Value};
use crate::state::AppState;

pub fn youtube_router() -> Router<AppState> {
    Router::new()
        .route("/youtube/generate-thumbnail", post(generate_thumbnail))
        .route("/youtube/generate-titles", post(generate_titles))
        .route("/youtube/refine-titles", post(refine_titles))
        .route("/youtube/generate-description", post(generate_description))
        .layer(DefaultBodyLimit::max(200 * 1024 * 1024))
}

// ---------------------------------------------------------------------------
// Multipart helpers — the generate-* endpoints accept multipart form data so
// callers can attach video files, face images and background images in
// addition to plain text fields (topic, transcript, style, count, ...).
// ---------------------------------------------------------------------------

struct FilePart {
    name: String,
    #[allow(dead_code)]
    filename: String,
    mime: String,
    bytes: Vec<u8>,
}

struct MultiForm {
    fields: HashMap<String, String>,
    files: Vec<FilePart>,
}

impl MultiForm {
    fn text(&self, name: &str) -> Option<&str> {
        self.fields.get(name).map(|s| s.as_str()).filter(|s| !s.is_empty())
    }
    fn file(&self, name: &str) -> Option<&FilePart> {
        self.files.iter().find(|f| f.name == name && !f.bytes.is_empty())
    }
}

async fn read_multipart(mut mp: Multipart) -> Result<MultiForm, (StatusCode, Json<Value>)> {
    let mut fields = HashMap::new();
    let mut files = Vec::new();
    while let Some(field) = mp.next_field().await.map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))))? {
        let name = field.name().unwrap_or("").to_string();
        let mime = field.content_type().unwrap_or("").to_string();
        // Text fields have no content type (or text/plain); everything else is a file.
        let is_file = !mime.is_empty() && mime != "text/plain";
        if is_file {
            let filename = field.file_name().unwrap_or("file").to_string();
            let bytes = field.bytes().await.map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))))?.to_vec();
            if bytes.len() > 150 * 1024 * 1024 {
                return Err((StatusCode::PAYLOAD_TOO_LARGE, Json(json!({"error": "Uploaded file exceeds the 150MB limit"}))));
            }
            files.push(FilePart { name, filename, mime, bytes });
        } else {
            let text = field.text().await.map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))))?;
            fields.insert(name, text);
        }
    }
    Ok(MultiForm { fields, files })
}

fn resolve_gemini_key(api_key: Option<&str>) -> Result<String, (StatusCode, Json<Value>)> {
    api_key
        .filter(|k| !k.trim().is_empty())
        .map(String::from)
        .or_else(|| std::env::var("GEMINI_API_KEY").ok().filter(|k| !k.trim().is_empty()))
        .ok_or_else(|| (StatusCode::BAD_REQUEST, Json(json!({"error": "Gemini API key required"}))))
}

fn resolve_text_model(llm_provider: Option<&str>) -> String {
    match llm_provider {
        Some(m) if !m.trim().is_empty() => m.to_string(),
        _ => std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-3.1-flash-lite".into()),
    }
}

fn mime_for_filename(filename: &str) -> String {
    let lower = filename.to_lowercase();
    if lower.ends_with(".mov") { "video/quicktime".into() }
    else if lower.ends_with(".webm") { "video/webm".into() }
    else if lower.ends_with(".mkv") { "video/x-matroska".into() }
    else if lower.ends_with(".mpg") || lower.ends_with(".mpeg") { "video/mpeg".into() }
    else if lower.ends_with(".avi") { "video/x-msvideo".into() }
    else { "video/mp4".into() }
}

/// Extract a JSON object from a model response that may be wrapped in code fences
/// or surrounded by prose.
pub(crate) fn parse_json_block(text: &str) -> Result<Value, String> {
    let clean = text.trim().trim_start_matches("```json").trim_start_matches("```")
        .trim_end_matches("```").trim();
    let start = clean.find('{').ok_or_else(|| "No JSON object found in model response".to_string())?;
    let end = clean.rfind('}').ok_or_else(|| "No JSON object found in model response".to_string())?;
    serde_json::from_str(&clean[start..=end]).map_err(|e| e.to_string())
}

/// Upload a video to the Gemini Files API (resumable upload) and wait until it
/// is ACTIVE, returning (file_uri, file_name) for use as a content part.
async fn gemini_upload_video(api_key: &str, display_name: &str, mime: &str, bytes: Vec<u8>) -> Result<(String, String), String> {
    let client = reqwest::Client::new();
    let total = bytes.len();

    let init = client
        .post(format!("https://generativelanguage.googleapis.com/upload/v1beta/files?key={}", api_key))
        .header("X-Goog-Upload-Protocol", "resumable")
        .header("X-Goog-Upload-Command", "start")
        .header("X-Goog-Upload-Header-Content-Length", total.to_string())
        .header("X-Goog-Upload-Header-Content-Type", mime)
        .json(&json!({"file": {"display_name": display_name}}))
        .send().await
        .map_err(|e| format!("Gemini upload init failed: {}", e))?;

    let upload_url = init.headers().get("X-Goog-Upload-URL")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .ok_or_else(|| format!("Gemini upload init failed (status {})", init.status()))?;

    let up = client.put(&upload_url)
        .header("Content-Length", total.to_string())
        .header("Content-Type", mime)
        .header("X-Goog-Upload-Protocol", "resumable")
        .header("X-Goog-Upload-Command", "upload, finalize")
        .body(bytes)
        .send().await
        .map_err(|e| format!("Gemini upload failed: {}", e))?;

    let body: Value = up.json().await.map_err(|e| format!("Gemini upload parse failed: {}", e))?;
    let name = body.get("file")
        .and_then(|f| f.get("name"))
        .and_then(|n| n.as_str())
        .map(String::from)
        .ok_or_else(|| format!("Gemini upload response missing file name: {}", body))?;
    let uri = format!("https://generativelanguage.googleapis.com/v1beta/{}", name);

    for _ in 0..30 {
        let info = client
            .get(format!("https://generativelanguage.googleapis.com/v1beta/{}?key={}", name, api_key))
            .send().await
            .map_err(|e| e.to_string())?;
        let body: Value = info.json().await.unwrap_or_else(|_| json!({}));
        match body.get("state").and_then(|s| s.as_str()) {
            Some("ACTIVE") => return Ok((uri, name)),
            Some("FAILED") => return Err("Gemini rejected the uploaded video".to_string()),
            _ => tokio::time::sleep(Duration::from_secs(2)).await,
        }
    }
    Err("Timed out waiting for Gemini to process the video".to_string())
}

// ---------------------------------------------------------------------------
// Thumbnails — multi-variant generation with optional face/background images
// ---------------------------------------------------------------------------

async fn generate_thumbnail(
    State(_state): State<AppState>,
    multipart: Multipart,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let form = read_multipart(multipart).await?;
    let gemini_key = resolve_gemini_key(form.text("api_key"))?;

    let title = form.text("title").or_else(|| form.text("topic")).unwrap_or("").to_string();
    if title.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "A video title or topic is required"}))));
    }

    // Text models (e.g. flash-lite) cannot generate images — the thumbnail needs
    // a Gemini image model. Prefer the explicit `image_model` field, then any
    // `llm_provider` value, then GEMINI_THUMBNAIL_MODEL / the built-in default.
    let image_model = match form.text("image_model") {
        Some(m) if !m.trim().is_empty() => m.to_string(),
        _ => match form.text("llm_provider") {
            Some(m) if !m.trim().is_empty() => m.to_string(),
            _ => std::env::var("GEMINI_THUMBNAIL_MODEL").unwrap_or_else(|_| "gemini-3.1-flash-image-preview".into()),
        },
    };

    let style_prompt = match form.text("style").unwrap_or("viral") {
        "comparison" => "before/after split comparison thumbnail style",
        "text" => "bold text overlay with high contrast",
        "minimal" => "clean minimal design with one focal element",
        "step" => "numbered step-by-step layout",
        _ => "viral YouTube reaction-style thumbnail with face and bold text",
    };

    let count = form.text("count").and_then(|c| c.parse::<usize>().ok()).unwrap_or(3).clamp(1, 4);
    let video_context = form.text("video_context").unwrap_or("");
    let extra_prompt = form.text("extra_prompt").unwrap_or("");

    let face = form.file("face_image");
    let bg = form.file("bg_image");

    let mut prompt = format!(
        "Generate a professional, eye-catching YouTube thumbnail image.\n\n\
         VIDEO TITLE (for reference — do NOT put the full title on the thumbnail): \"{}\"\n\n",
        title
    );
    if !video_context.trim().is_empty() {
        prompt.push_str(&format!("VIDEO CONTEXT (use this to understand the video and design a relevant thumbnail):\n{}\n\n", video_context));
    }
    prompt.push_str(
        "TEXT ON THE THUMBNAIL:\n\
         - Create a SHORT visual hook: 1 to 5 words maximum, based on the title and context\n\
         - The thumbnail text should COMPLEMENT the title, not repeat it\n\
         - Use ALL CAPS for maximum impact, split into 2-3 lines\n",
    );
    if !extra_prompt.trim().is_empty() {
        prompt.push_str(&format!(
            "MANDATORY USER INSTRUCTIONS (MUST follow these exactly — they override any default behavior):\n{}\n\n",
            extra_prompt
        ));
    }
    prompt.push_str(&format!(
        "DESIGN REQUIREMENTS:\n- {}\n- The text MUST be large, bold, and high-contrast (readable at small sizes)\n\
         - Use vibrant, eye-catching colors that match the video's mood\n- Professional YouTube thumbnail aesthetic\n\
         - Clean composition — text and face/subject as clear focal points\n- NO clutter, NO small text, NO watermarks\n\
         - The image should be exactly 16:9.",
        style_prompt
    ));
    if face.is_some() {
        prompt.push_str("\n- Include the provided face/person prominently with an exaggerated expression (surprise, excitement, shock)");
    }
    if bg.is_some() {
        prompt.push_str("\n- Use the provided background image as the base/backdrop");
    }

    let client = reqwest::Client::new();
    let mut thumbnails: Vec<Value> = Vec::new();

    for i in 0..count {
        let mut parts: Vec<Value> = Vec::new();
        if let Some(f) = face {
            let b64 = base64::engine::general_purpose::STANDARD.encode(&f.bytes);
            parts.push(json!({"inlineData": {"mimeType": f.mime, "data": b64}}));
        }
        if let Some(b) = bg {
            let b64 = base64::engine::general_purpose::STANDARD.encode(&b.bytes);
            parts.push(json!({"inlineData": {"mimeType": b.mime, "data": b64}}));
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
            image_model, gemini_key
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
            return Err((StatusCode::BAD_GATEWAY, Json(json!({
                "error": format!("Gemini returned no image for variant {} — the model '{}' cannot generate images. Use a Gemini image model (e.g. gemini-3.1-flash-image-preview).", i + 1, image_model)
            }))));
        }

        let doc = parse_json_block(&text_out).unwrap_or_else(|_| json!({"prompt": text_out, "design_notes": ""}));
        let prompt_text = doc.get("prompt").and_then(|v| v.as_str()).unwrap_or(text_out.as_str());
        let design_notes = doc.get("design_notes").and_then(|v| v.as_str()).unwrap_or("");

        thumbnails.push(json!({
            "index": i,
            "image_url": format!("data:{};base64,{}", mime, image_b64),
            "mime_type": mime,
            "prompt": prompt_text,
            "design_notes": design_notes,
        }));
    }

    Ok(Json(json!({
        "thumbnails": thumbnails,
        "count": thumbnails.len(),
        "image_url": thumbnails.first().and_then(|t| t.get("image_url")).cloned().unwrap_or(Value::Null),
    })))
}

// ---------------------------------------------------------------------------
// Titles — video-aware (accepts an uploaded video, transcript text, or topic)
// ---------------------------------------------------------------------------

async fn generate_titles(
    State(_state): State<AppState>,
    multipart: Multipart,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let form = read_multipart(multipart).await?;
    let gemini_key = resolve_gemini_key(form.text("api_key"))?;
    let text_model = resolve_text_model(form.text("llm_provider"));

    let topic = form.text("topic").unwrap_or("").to_string();
    let transcript = form.text("transcript").unwrap_or("").to_string();
    let video_file = form.file("video_file");

    if topic.trim().is_empty() && transcript.trim().is_empty() && video_file.is_none() {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Provide a topic, a transcript, or a video file"}))));
    }

    let tone_desc = match form.text("tone").unwrap_or("viral") {
        "educational" => "educational how-to style",
        "story" => "story-driven curiosity gap style",
        "controversial" => "controversial debate-sparking style",
        "listicle" => "listicle top-N style",
        _ => "viral clickbaity style",
    };
    let count = form.text("count").and_then(|c| c.parse::<usize>().ok()).unwrap_or(10).clamp(5, 20);

    let mut context_block = String::new();
    if !transcript.trim().is_empty() {
        context_block.push_str(&format!("TRANSCRIPT:\n{}\n", transcript));
    }
    if !topic.trim().is_empty() {
        context_block.push_str(&format!("VIDEO TOPIC: {}\n", topic));
    }

    let prompt = format!(
        "You are a YouTube title expert who creates viral, click-worthy titles.\n\n\
         Analyze the video and its transcript, then suggest {} YouTube titles that would maximize CTR (click-through rate).\n\n\
         {}\n\
         RULES:\n\
         - Titles must be under 70 characters\n\
         - Use power words, curiosity gaps, and emotional triggers\n\
         - Mix styles: how-to, listicle, story-driven, controversial, question-based ({})\n\
         - Make them specific to the actual content, not generic\n\
         - Include numbers where appropriate\n\
         - Titles should be in the SAME LANGUAGE as the video\n\n\
         Also provide a brief summary of the video content (2-3 sentences), and pick the TOP 2 most recommended titles with concise reasons (reference them by their 0-based index in the titles array).\n\n\
         OUTPUT JSON: {{\"titles\": [\"title1\", \"title2\", ...], \"transcript_summary\": \"Brief summary...\", \"language\": \"...\", \"recommended\": [{{\"index\": 0, \"reason\": \"Why this title is best...\"}}]}}",
        count, context_block, tone_desc
    );

    let client = reqwest::Client::new();
    let mut parts: Vec<Value> = Vec::new();
    if let Some(f) = video_file {
        let mime = mime_for_filename(&f.filename);
        let (uri, _name) = gemini_upload_video(&gemini_key, &f.filename, &mime, f.bytes.clone())
            .await
            .map_err(|e| (StatusCode::BAD_GATEWAY, Json(json!({"error": e}))))?;
        parts.push(json!({"fileData": {"mimeType": mime, "fileUri": uri}}));
    }
    parts.push(json!({"text": prompt}));

    let body = json!({
        "systemInstruction": {"parts": [{"text": "You are a YouTube title optimization expert. Return ONLY valid JSON."}]},
        "contents": [{"role": "user", "parts": parts}],
        "generationConfig": {"temperature": 0.8, "maxOutputTokens": 4096, "responseMimeType": "application/json"}
    });

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        text_model, gemini_key
    );
    let resp = client.post(&url).json(&body)
        .timeout(Duration::from_secs(120))
        .send().await
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(json!({"error": format!("Gemini error: {}", e)}))))?;

    let text = resp.json::<Value>().await
        .ok().and_then(|r| r.pointer("/candidates/0/content/parts/0/text").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .ok_or_else(|| (StatusCode::BAD_GATEWAY, Json(json!({"error": "Empty Gemini response"}))))?;

    let mut result = parse_json_block(&text)
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(json!({"error": format!("Parse error: {}", e)}))))?;

    if let Some(titles) = result.get_mut("titles").and_then(|t| t.as_array_mut()) {
        titles.truncate(count);
    }
    result["language"] = json!(result.get("language").and_then(|v| v.as_str()).unwrap_or("en"));
    result["transcript_summary"] = json!(result.get("transcript_summary").and_then(|v| v.as_str()).unwrap_or(""));
    result["segments"] = json!([]);
    result["video_duration"] = json!(null);

    Ok(Json(result))
}

// ---------------------------------------------------------------------------
// Title refinement (JSON-only, no file uploads)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct RefineTitlesRequest {
    topic: String,
    current_titles: Vec<String>,
    instruction: String,
    llm_provider: Option<String>,
    api_key: Option<String>,
}

async fn refine_titles(
    State(_state): State<AppState>,
    Json(req): Json<RefineTitlesRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let gemini_key = resolve_gemini_key(req.api_key.as_deref())?;
    let text_model = resolve_text_model(req.llm_provider.as_deref());

    let titles_str = req.current_titles.iter()
        .enumerate()
        .map(|(i, t)| format!("{}. {}", i + 1, t))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!(
        "Current YouTube titles for topic \"{}\":\n{}\n\nUser instruction: \"{}\"\n\n\
         Refine these titles based on the instruction. Return the full refined list. \
         Return JSON: {{\"titles\": [\"refined title 1\", ...], \"response\": \"friendly confirmation message\"}}",
        req.topic, titles_str, req.instruction
    );

    let body = json!({
        "systemInstruction": {"parts": [{"text": "You are a YouTube title optimization expert. Return ONLY valid JSON."}]},
        "contents": [{"role": "user", "parts": [{"text": prompt}]}],
        "generationConfig": {"temperature": 0.7, "maxOutputTokens": 2048, "responseMimeType": "application/json"}
    });

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        text_model, gemini_key
    );

    let client = reqwest::Client::new();
    let resp = client.post(&url).json(&body)
        .timeout(Duration::from_secs(30))
        .send().await
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(json!({"error": format!("Gemini error: {}", e)}))))?;

    let text = resp.json::<Value>().await
        .ok().and_then(|r| r.pointer("/candidates/0/content/parts/0/text").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .ok_or_else(|| (StatusCode::BAD_GATEWAY, Json(json!({"error": "Empty Gemini response"}))))?;

    let result = parse_json_block(&text)
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(json!({"error": format!("Parse error: {}", e)}))))?;

    Ok(Json(result))
}

// ---------------------------------------------------------------------------
// Descriptions — video-aware (accepts an uploaded video, transcript/URL text)
// ---------------------------------------------------------------------------

async fn generate_description(
    State(_state): State<AppState>,
    multipart: Multipart,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let form = read_multipart(multipart).await?;
    let gemini_key = resolve_gemini_key(form.text("api_key"))?;
    let text_model = resolve_text_model(form.text("llm_provider"));

    let topic = form.text("topic").unwrap_or("").to_string();
    let transcript = form.text("transcript").unwrap_or("").to_string();
    let video_url = form.text("video_url").unwrap_or("").to_string();
    let video_file = form.file("video_file");

    if topic.trim().is_empty() && transcript.trim().is_empty() && video_url.trim().is_empty() && video_file.is_none() {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Provide a topic, transcript, video URL, or video file"}))));
    }

    let mut context_block = String::new();
    if !transcript.trim().is_empty() {
        context_block.push_str(&format!("TRANSCRIPT:\n{}\n", transcript));
    }
    if !video_url.trim().is_empty() {
        context_block.push_str(&format!("VIDEO URL: {}\n", video_url));
    }
    if !topic.trim().is_empty() {
        context_block.push_str(&format!("VIDEO TOPIC: {}\n", topic));
    }

    let prompt = format!(
        "You are a YouTube SEO expert. Generate a complete YouTube video description with chapter markers.\n\n\
         VIDEO REFERENCE:\n{}\n\
         REQUIREMENTS:\n\
         1. Start with a compelling 2-3 sentence summary hook\n\
         2. Add relevant CTAs (subscribe, like, comment)\n\
         3. Generate YouTube CHAPTERS based on the video/transcript timestamps: first chapter MUST start at 0:00, minimum 3 chapters, each at least 10 seconds apart, concise descriptive titles\n\
         4. Add 5-10 relevant hashtags at the end\n\
         5. Keep the total description under 5000 characters\n\
         6. Write the description in the SAME LANGUAGE as the video\n\n\
         OUTPUT JSON: {{\"description\": \"full description text with formatting and chapters\", \"chapters\": [{{\"time\": \"0:00\", \"title\": \"Introduction\"}}]}}",
        context_block
    );

    let client = reqwest::Client::new();
    let mut parts: Vec<Value> = Vec::new();
    if let Some(f) = video_file {
        let mime = mime_for_filename(&f.filename);
        let (uri, _name) = gemini_upload_video(&gemini_key, &f.filename, &mime, f.bytes.clone())
            .await
            .map_err(|e| (StatusCode::BAD_GATEWAY, Json(json!({"error": e}))))?;
        parts.push(json!({"fileData": {"mimeType": mime, "fileUri": uri}}));
    }
    parts.push(json!({"text": prompt}));

    let body = json!({
        "systemInstruction": {"parts": [{"text": "You are a YouTube SEO and description expert. Return ONLY valid JSON."}]},
        "contents": [{"role": "user", "parts": parts}],
        "generationConfig": {"temperature": 0.6, "maxOutputTokens": 4096, "responseMimeType": "application/json"}
    });

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        text_model, gemini_key
    );
    let resp = client.post(&url).json(&body)
        .timeout(Duration::from_secs(120))
        .send().await
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(json!({"error": format!("Gemini error: {}", e)}))))?;

    let text = resp.json::<Value>().await
        .ok().and_then(|r| r.pointer("/candidates/0/content/parts/0/text").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .ok_or_else(|| (StatusCode::BAD_GATEWAY, Json(json!({"error": "Empty Gemini response"}))))?;

    let result = parse_json_block(&text)
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(json!({"error": format!("Parse error: {}", e)}))))?;

    Ok(Json(result))
}
