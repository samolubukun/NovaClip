use anyhow::Result;
use std::path::{Path, PathBuf};
use tracing::info;
use crate::pipeline::dedup::timestamp_to_seconds;

/// Build FFmpeg crop/scale filter string for a given aspect ratio
pub fn build_crop_filter(aspect_ratio: &str) -> String {
    match aspect_ratio {
        "9:16" => {
            // Smart center crop for vertical: take full height, crop width proportionally
            "scale=-1:1920:flags=lanczos,crop=1080:1920:(iw-1080)/2:0,setsar=1".to_string()
        }
        "1:1" => {
            "scale=-1:1080:flags=lanczos,crop=1080:1080:(iw-1080)/2:(ih-1080)/2,setsar=1".to_string()
        }
        "16:9" => {
            "scale=1920:-1:flags=lanczos,pad=1920:1080:(ow-iw)/2:(oh-ih)/2,setsar=1".to_string()
        }
        "4:3" => {
            "scale=-1:1080:flags=lanczos,crop=1440:1080:(iw-1440)/2:0,setsar=1".to_string()
        }
        _ => "".to_string(), // original — no filter
    }
}

/// Extract a clip from the source video using FFmpeg
/// If `skip_crop` is true, extracts at original aspect ratio (no crop filter).
pub async fn extract_clip(
    source: &Path,
    output_dir: &Path,
    clip_index: i32,
    start_time: &str,
    end_time: &str,
    aspect_ratio: &str,
    task_id: &str,
    skip_crop: bool,
) -> Result<PathBuf> {
    tokio::fs::create_dir_all(output_dir).await?;

    let start_secs = timestamp_to_seconds(start_time);
    let end_secs = timestamp_to_seconds(end_time);
    let duration = end_secs - start_secs;

    if duration <= 0.0 {
        anyhow::bail!("Clip duration <= 0: {} - {}", start_time, end_time);
    }

    let filename = format!("clip_{:02}_{}.mp4", clip_index + 1, &task_id[..8]);
    let output_path = output_dir.join(&filename);

    info!("Extracting clip {} [{} - {}] ({:.1}s)", clip_index + 1, start_time, end_time, duration);

    let mut args: Vec<String> = vec![
        "-y".into(),
        "-ss".into(), start_secs.to_string(),
        "-i".into(), source.to_str().unwrap().to_string(),
        "-t".into(), duration.to_string(),
    ];

    if !skip_crop {
        let crop_filter = build_crop_filter(aspect_ratio);
        if !crop_filter.is_empty() {
            args.extend(["-vf".into(), crop_filter]);
        }
    }

    args.extend([
        "-c:v".into(), "libx264".into(),
        "-preset".into(), "fast".into(),
        "-crf".into(), "18".into(),
        "-pix_fmt".into(), "yuv420p".into(),
        "-profile:v".into(), "high".into(),
        "-c:a".into(), "aac".into(),
        "-b:a".into(), "192k".into(),
        "-movflags".into(), "+faststart".into(),
        output_path.to_str().unwrap().to_string(),
    ]);

    let status = tokio::process::Command::new("ffmpeg")
        .args(&args)
        .status()
        .await?;

    if !status.success() {
        anyhow::bail!("FFmpeg clip extraction failed for clip {}", clip_index + 1);
    }

    info!("Clip {} extracted: {}", clip_index + 1, output_path.display());
    Ok(output_path)
}
