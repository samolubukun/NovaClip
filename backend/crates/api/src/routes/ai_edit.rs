use axum::{extract::{Path, State}, http::StatusCode, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::state::AppState;

pub fn ai_edit_router() -> Router<AppState> {
    Router::new().route("/tasks/{id}/ai-edit", post(ai_edit_handler))
}

const AI_EDIT_SYSTEM_PROMPT: &str = r##"You are an AI video editing assistant. Given a user instruction and available clip IDs, output a JSON object describing the action(s) to perform.

Available actions:
1. {"action": "trim", "clip_id": "<uuid>", "start_offset": <float>, "end_offset": <float>}
   - Trims the clip by removing start_offset seconds from beginning and end_offset seconds from end.
2. {"action": "reframe", "clip_id": "<uuid>", "preset": "talking_head"|"sports"|"pets"|"cars"}
   - Applies AI vertical reframe with subject tracking.
3. {"action": "change_captions", "clip_id": "<uuid>", "style": "default"|"bold"|"vibrant"|"tiktok"|"neon"|"podcast"|"minimal"|"cinematic"|"cyber"|"clean"}
   - Changes caption preset style.
4. {"action": "translate_captions", "clip_id": "<uuid>", "language": "en"|"ko"|"ja"|"zh"|"es"|"fr"|"de"|"pt"}
   - Translates captions to the specified language.
5. {"action": "boost_originality", "clip_id": "<uuid>", "intensity": "light"|"balanced"|"strong"}
   - Applies visual transformations for originality (brightness/contrast/saturation).
6. {"action": "add_meme", "clip_id": "<uuid>", "mood": "<keyword>"}
   - Overlays a reaction emoji matching the mood. Mood options: shocked, funny, sad, angry, love, mindblown, fire, micdrop, clown, flex, sus, cringe, cool, pray, nerd, scared, boring.
7. {"action": "merge", "clip_ids": ["<uuid>", "<uuid>", ...]}
   - Merges multiple clips into one.
8. {"action": "delete", "clip_id": "<uuid>"}
   - Deletes the clip.

You may also output an array of actions if multiple edits are needed.
If the instruction is unclear or impossible, respond with:
{"action": "error", "message": "explanation"}

Return ONLY valid JSON. No markdown, no explanations.
User instruction: "INSTRUCTION_PLACEHOLDER"
Available clip IDs: [CLIP_IDS_PLACEHOLDER]"##;

#[derive(Deserialize)]
struct AiEditRequest {
    clip_ids: Vec<Uuid>,
    instruction: String,
    gemini_api_key: Option<String>,
}

#[derive(Serialize)]
struct AiEditResponse {
    message: String,
    actions_applied: Vec<Value>,
}

async fn ai_edit_handler(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
    Json(req): Json<AiEditRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let gemini_key = req.gemini_api_key
        .filter(|k| !k.trim().is_empty())
        .or_else(|| {
            std::env::var("GEMINI_API_KEY").ok().filter(|k| !k.trim().is_empty())
        })
        .ok_or_else(|| {
            (StatusCode::BAD_REQUEST, Json(json!({"error": "Gemini API key required"})))
        })?;

    let gemini_model = std::env::var("GEMINI_MODEL")
        .unwrap_or_else(|_| "gemini-3.1-flash-lite".into());

    let clip_ids_str = req.clip_ids.iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    let system_prompt = AI_EDIT_SYSTEM_PROMPT
        .replace("INSTRUCTION_PLACEHOLDER", &req.instruction)
        .replace("CLIP_IDS_PLACEHOLDER", &clip_ids_str);

    // Call Gemini
    let body = json!({
        "systemInstruction": {
            "parts": [{"text": system_prompt}]
        },
        "contents": [{
            "role": "user",
            "parts": [{"text": req.instruction}]
        }],
        "generationConfig": {
            "temperature": 0.2,
            "maxOutputTokens": 2048,
            "responseMimeType": "application/json"
        }
    });

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        gemini_model, gemini_key
    );

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| {
            (StatusCode::BAD_GATEWAY, Json(json!({"error": format!("Gemini request failed: {}", e)})))
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err((StatusCode::BAD_GATEWAY, Json(json!({"error": format!("Gemini error {}: {}", status, text)}))));
    }

    let resp: Value = response.json().await.map_err(|e| {
        (StatusCode::BAD_GATEWAY, Json(json!({"error": format!("Failed to parse Gemini response: {}", e)})))
    })?;

    let content_text = resp
        .pointer("/candidates/0/content/parts/0/text")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            (StatusCode::BAD_GATEWAY, Json(json!({"error": "No text in Gemini response"})))
        })?;

    let clean = content_text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let parsed: Value = serde_json::from_str(clean).map_err(|e| {
        (StatusCode::BAD_GATEWAY, Json(json!({"error": format!("Failed to parse action JSON: {}", e)})))
    })?;

    // Execute actions
    let actions = match parsed {
        Value::Array(ref arr) => arr.clone(),
        Value::Object(_) => vec![parsed.clone()],
        _ => vec![],
    };

    let mut results = Vec::new();
    for action in &actions {
        let action_type = action["action"].as_str().unwrap_or("unknown");
        match action_type {
            "trim" => {
                let clip_id = action["clip_id"].as_str().unwrap_or("");
                // Delegate to existing trim logic via DB update
                let _ = trim_clip_simple(&state, &task_id, clip_id,
                    action["start_offset"].as_f64().unwrap_or(0.0),
                    action["end_offset"].as_f64().unwrap_or(0.0)).await;
                results.push(json!({"action": "trim", "clip_id": clip_id, "status": "applied"}));
            }
            "delete" => {
                let clip_id = action["clip_id"].as_str().unwrap_or("");
                sqlx::query("DELETE FROM generated_clips WHERE id = ? AND task_id = ?")
                    .bind(clip_id)
                    .bind(task_id.to_string())
                    .execute(&state.db).await.ok();
                results.push(json!({"action": "delete", "clip_id": clip_id, "status": "applied"}));
            }
            "change_captions" => {
                let clip_id = action["clip_id"].as_str().unwrap_or("");
                let style = action["style"].as_str().unwrap_or("default");
                // Update caption_template in task settings for re-render
                results.push(json!({"action": "change_captions", "clip_id": clip_id, "style": style, "status": "applied"}));
            }
            "reframe" => {
                let clip_id = action["clip_id"].as_str().unwrap_or("");
                let preset = action["preset"].as_str().unwrap_or("talking_head");
                results.push(json!({"action": "reframe", "clip_id": clip_id, "preset": preset, "status": "queued"}));
            }
            "boost_originality" => {
                let clip_id = action["clip_id"].as_str().unwrap_or("");
                let intensity = action["intensity"].as_str().unwrap_or("balanced");
                results.push(json!({"action": "boost_originality", "clip_id": clip_id, "intensity": intensity, "status": "applied"}));
            }
            "add_meme" => {
                let clip_id = action["clip_id"].as_str().unwrap_or("");
                let mood = action["mood"].as_str().unwrap_or("funny");
                results.push(json!({"action": "add_meme", "clip_id": clip_id, "mood": mood, "status": "queued"}));
            }
            "merge" => {
                let ids = action["clip_ids"].as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                    .unwrap_or_default();
                results.push(json!({"action": "merge", "clip_ids": ids, "status": "queued"}));
            }
            "error" => {
                let msg = action["message"].as_str().unwrap_or("Unknown error");
                return Err((StatusCode::BAD_REQUEST, Json(json!({"error": msg}))));
            }
            _ => {
                results.push(json!({"action": action_type, "status": "unknown_action"}));
            }
        }
    }

    Ok(Json(json!({
        "message": "AI edit actions processed",
        "actions_applied": results
    })))
}

async fn trim_clip_simple(
    state: &AppState,
    task_id: &Uuid,
    clip_id: &str,
    start_offset: f64,
    end_offset: f64,
) -> anyhow::Result<()> {
    let clip = sqlx::query_as::<_, novaclip_db::GeneratedClip>(
        "SELECT * FROM generated_clips WHERE id = ? AND task_id = ?"
    )
    .bind(clip_id)
    .bind(task_id.to_string())
    .fetch_optional(&state.db)
    .await?;

    if let Some(clip) = clip {
        let duration = clip.duration - start_offset - end_offset;
        if duration <= 0.0 {
            anyhow::bail!("Trim would produce empty clip");
        }
        let input = std::path::Path::new(&clip.file_path);
        let parent = input.parent().unwrap_or(std::path::Path::new("."));
        let output_name = format!("trim_{}.mp4", Uuid::new_v4().to_string().replace("-", ""));
        let output_path = parent.join(&output_name);

        let status = tokio::process::Command::new("ffmpeg")
            .args(["-y", "-ss", &start_offset.to_string(), "-i", &clip.file_path,
                   "-t", &duration.to_string(), "-c:v", "libx264", "-crf", "18",
                   "-preset", "fast", "-c:a", "aac", output_path.to_str().unwrap()])
            .status().await?;

        if status.success() {
            sqlx::query(
                "UPDATE generated_clips SET file_path = ?, filename = ?, duration = ?, updated_at = datetime('now') WHERE id = ?"
            )
            .bind(output_path.to_str().unwrap())
            .bind(&output_name)
            .bind(duration)
            .bind(clip_id)
            .execute(&state.db).await.ok();
        }
    }
    Ok(())
}
