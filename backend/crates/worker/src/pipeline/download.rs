use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Download a YouTube video using yt-dlp and return the local file path.
pub async fn download_youtube(url: &str, temp_dir: &str) -> Result<PathBuf> {
    tokio::fs::create_dir_all(temp_dir).await?;

    // Extract video ID for filename
    let video_id = extract_video_id(url).unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let output_template = format!("{}/{}.%(ext)s", temp_dir, video_id);

    info!("Downloading YouTube video: {}", url);

    let cur_ytdlp = std::env::current_dir().unwrap_or_default().join("yt-dlp.exe");
    let temp_ytdlp = Path::new(temp_dir).join("yt-dlp.exe");

    let ytdlp_cmd = if cur_ytdlp.exists() {
        cur_ytdlp.to_string_lossy().to_string()
    } else if temp_ytdlp.exists() {
        temp_ytdlp.to_string_lossy().to_string()
    } else {
        "yt-dlp".to_string()
    };

    info!("Using yt-dlp binary at: {}", ytdlp_cmd);

    let output = tokio::process::Command::new(&ytdlp_cmd)
        .args([
            "--format", "bestvideo*+bestaudio/best",
            "--merge-output-format", "mp4",
            "--output", &output_template,
            "--no-playlist",
            "--retries", "5",
            "--fragment-retries", "5",
            "--http-chunk-size", "10M",
            "--quiet",
            "--no-warnings",
            "--no-check-certificate",
            "--add-header", "User-Agent:Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            url,
        ])
        .output()
        .await
        .context("Failed to spawn yt-dlp. Is it installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("yt-dlp failed: {}", stderr);
    }

    // Find the downloaded file
    let expected_mp4 = PathBuf::from(format!("{}/{}.mp4", temp_dir, video_id));
    if expected_mp4.exists() {
        info!("Download complete: {}", expected_mp4.display());
        return Ok(expected_mp4);
    }

    // Scan temp dir for the video
    let mut entries = tokio::fs::read_dir(temp_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if let Some(stem) = path.file_stem() {
            if stem.to_string_lossy() == video_id {
                return Ok(path);
            }
        }
    }

    anyhow::bail!("Downloaded file not found in {}", temp_dir)
}

/// Resolve an uploaded video path (upload://filename -> actual path)
pub fn resolve_upload_path(url: &str, temp_dir: &str) -> PathBuf {
    let filename = url.strip_prefix("upload://").unwrap_or(url);
    PathBuf::from(format!("{}/uploads/{}", temp_dir, filename))
}

fn extract_video_id(url: &str) -> Option<String> {
    if let Ok(parsed) = url::Url::parse(url) {
        if parsed.host_str() == Some("youtu.be") {
            return parsed.path_segments()?.next().map(|s| s.to_string());
        }
        for (key, val) in parsed.query_pairs() {
            if key == "v" {
                return Some(val.to_string());
            }
        }
    }
    None
}

/// Extract audio from video to mono MP3 for transcription
pub async fn extract_audio(video_path: &Path, temp_dir: &str) -> Result<PathBuf> {
    let stem = video_path.file_stem().unwrap_or_default().to_string_lossy();
    let audio_path = PathBuf::from(format!("{}/{}_audio.mp3", temp_dir, stem));

    if audio_path.exists() && audio_path.metadata()?.len() > 0 {
        info!("Using cached audio: {}", audio_path.display());
        return Ok(audio_path);
    }

    info!("Extracting audio for transcription...");
    let status = tokio::process::Command::new("ffmpeg")
        .args([
            "-y", "-i", video_path.to_str().unwrap(),
            "-vn", "-ac", "1", "-ar", "16000",
            "-b:a", "64k", "-f", "mp3",
            audio_path.to_str().unwrap(),
        ])
        .status().await?;

    if !status.success() {
        anyhow::bail!("ffmpeg audio extraction failed");
    }
    Ok(audio_path)
}

/// Get video duration in seconds via ffprobe
pub async fn get_video_duration(video_path: &Path) -> Result<f64> {
    let output = tokio::process::Command::new("ffprobe")
        .args([
            "-v", "error",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
            video_path.to_str().unwrap(),
        ])
        .output().await?;
    let s = String::from_utf8_lossy(&output.stdout);
    Ok(s.trim().parse::<f64>().unwrap_or(0.0))
}
