use anyhow::{Context, Result};
use std::path::Path;
use tracing::info;

const UPLOADPOST_BASE: &str = "https://api.upload-post.com/api";

/// Fetch the list of Upload-Post profiles for a BYOK key and return the first
/// usable profile username, preferring one connected to YouTube. Mirrors the
/// OpenShorts /api/social/user resolver.
pub async fn resolve_profile(api_key: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/uploadposts/users", UPLOADPOST_BASE))
        .header("Authorization", format!("Apikey {}", api_key))
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .context("Upload-Post profiles request failed")?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("Upload-Post profiles error {}: {}", status, text);
    }
    let body: serde_json::Value = resp.json().await.context("Upload-Post profiles parse failed")?;
    let profiles = body.get("profiles").and_then(|v| v.as_array()).cloned().unwrap_or_default();

    let mut fallback: Option<String> = None;
    for p in &profiles {
        let username = p.get("username").and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
        if username.trim().is_empty() {
            continue;
        }
        if fallback.is_none() {
            fallback = Some(username.clone());
        }
        let socials = p.get("social_accounts");
        let youtube_connected = match socials {
            Some(s) => match s.get("youtube") {
                Some(v) if v.is_object() => !v.as_object().map(|o| o.is_empty()).unwrap_or(true),
                Some(v) if v.is_string() => !v.as_str().unwrap_or("").is_empty(),
                _ => false,
            },
            None => false,
        };
        if youtube_connected {
            return Ok(username);
        }
    }
    fallback.ok_or_else(|| {
        anyhow::anyhow!(
            "No Upload-Post profiles found for this key — create one at app.upload-post.com"
        )
    })
}

/// Publish a video to one or more social platforms via Upload-Post
/// (multipart upload, async). Matches the OpenShorts /api/social/post format.
/// Supported targets: youtube, tiktok, instagram (and more — see Upload-Post
/// VideoPlatformEnum). Platform-specific titles fall back to `title`.
pub async fn publish_video(
    api_key: &str,
    profile_username: &str,
    video_path: &Path,
    title: &str,
    description: &str,
    platforms: &[String],
) -> Result<serde_json::Value> {
    let client = reqwest::Client::new();
    let file_bytes = tokio::fs::read(video_path)
        .await
        .with_context(|| format!("Failed to read video file: {}", video_path.display()))?;
    let filename = video_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("video.mp4")
        .to_string();
    let video_part = reqwest::multipart::Part::bytes(file_bytes)
        .file_name(filename)
        .mime_str("video/mp4")
        .context("Failed to build video multipart part")?;

    let targets: Vec<String> = if platforms.is_empty() {
        vec!["youtube".to_string()]
    } else {
        platforms.to_vec()
    };

    let mut form = reqwest::multipart::Form::new()
        .text("user", profile_username.to_string())
        .text("title", title.to_string())
        .text("async_upload", "true")
        .text("privacyStatus", "public")
        .part("video", video_part);

    for p in &targets {
        form = form.text("platform[]", p.clone());
    }

    // Platform-specific fields (all fall back to `title`/`description`).
    if targets.iter().any(|p| p == "youtube") {
        form = form
            .text("youtube_title", title.to_string())
            .text("youtube_description", description.to_string())
            .text("containsSyntheticMedia", "true");
    }
    if targets.iter().any(|p| p == "tiktok") {
        form = form
            .text("tiktok_title", title.to_string())
            .text("description", description.to_string())
            .text("is_aigc", "true");
    }
    if targets.iter().any(|p| p == "instagram") {
        form = form
            .text("instagram_title", title.to_string())
            .text("media_type", "REELS");
    }
    if targets.iter().any(|p| p == "linkedin") {
        form = form.text("linkedin_title", title.to_string());
    }

    let resp = client
        .post(format!("{}/upload", UPLOADPOST_BASE))
        .header("Authorization", format!("Apikey {}", api_key))
        .multipart(form)
        .timeout(std::time::Duration::from_secs(600))
        .send()
        .await
        .context("Upload-Post upload request failed")?;

    if !resp.status().is_success() && resp.status() != 202 {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("Upload-Post API error {}: {}", status, text);
    }
    let body: serde_json::Value = resp.json().await.unwrap_or_else(|_| serde_json::json!({}));
    info!("Upload-Post publish submitted for '{}' -> {:?}: {}", title, targets, body);
    Ok(body)
}
