use anyhow::Result;
use std::path::Path;
use tracing::info;

/// Parse originality setting. Supports presets and custom format: "custom:B:C:S"
fn parse_originality(intensity: &str) -> Option<(f64, f64, f64)> {
    match intensity {
        "light" => Some((0.03, 1.03, 1.05)),
        "balanced" => Some((0.05, 1.08, 1.10)),
        "strong" => Some((0.08, 1.12, 1.15)),
        _ if intensity.starts_with("custom:") => {
            let parts: Vec<&str> = intensity[7..].split(':').collect();
            if parts.len() == 3 {
                let b = parts[0].parse::<f64>().ok()?;
                let c = parts[1].parse::<f64>().ok()?;
                let s = parts[2].parse::<f64>().ok()?;
                Some((b, c, s))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Apply originality boost filters (brightness/contrast/saturation) via ffmpeg.
/// Returns the path to the processed file.
pub async fn apply_originality_boost(
    input: &Path,
    output: &Path,
    intensity: &str,
) -> Result<()> {
    let (brightness, contrast, saturation) = match parse_originality(intensity) {
        Some(v) => v,
        None => return Ok(()), // "none" or unknown — skip
    };

    info!(
        "Applying originality boost ({}): b={}, c={}, s={} on {}",
        intensity, brightness, contrast, saturation, input.display()
    );

    let filter = format!(
        "eq=brightness={}:contrast={}:saturation={},unsharp=3:3:0.5:3:3:0.0",
        brightness, contrast, saturation
    );

    let status = tokio::process::Command::new("ffmpeg")
        .args([
            "-y", "-i",
            input.to_str().unwrap(),
            "-vf", &filter,
            "-c:v", "libx264", "-preset", "fast", "-crf", "18",
            "-pix_fmt", "yuv420p",
            "-c:a", "aac", "-b:a", "192k",
            "-movflags", "+faststart",
            output.to_str().unwrap(),
        ])
        .status()
        .await?;

    if !status.success() {
        anyhow::bail!("Originality boost ffmpeg failed");
    }

    info!("Originality boost complete: {}", output.display());
    Ok(())
}
