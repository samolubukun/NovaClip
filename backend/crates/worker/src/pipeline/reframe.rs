use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tracing::info;

/// Find the venv Python interpreter relative to the reframe package dir.
fn find_venv_python(reframe_dir: &Path) -> PathBuf {
    let venv_dir = reframe_dir.join("venv");
    if cfg!(target_os = "windows") {
        venv_dir.join("Scripts").join("python.exe")
    } else {
        venv_dir.join("bin").join("python")
    }
}

/// Probe the video's FPS via ffprobe.
async fn probe_fps(input: &Path) -> Result<f64> {
    let out = tokio::process::Command::new("ffprobe")
        .args([
            "-v", "0",
            "-of", "csv=p=0",
            "-select_streams", "v:0",
            "-show_entries", "stream=r_frame_rate",
            input.to_str().unwrap(),
        ])
        .output()
        .await?;
    let rate_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
    // ffprobe returns e.g. "30000/1001" or "30/1"
    let parts: Vec<f64> = rate_str.split('/').filter_map(|s| s.parse().ok()).collect();
    if parts.len() == 2 && parts[1] > 0.0 {
        Ok(parts[0] / parts[1])
    } else if parts.len() == 1 {
        Ok(parts[0])
    } else {
        Ok(30.0) // fallback
    }
}

/// Run vertical reframe (AI subject tracking) on a clip via Python subprocess.
/// `frame_skip` > 1 decimates frames before reframe for speed (e.g. 3 = process every 3rd frame).
pub async fn apply_vertical_reframe(
    input: &Path,
    output: &Path,
    preset: &str,
    frame_skip: u32,
) -> Result<()> {
    info!(
        "Running vertical reframe on {} -> {} (preset: {}, skip={})",
        input.display(),
        output.display(),
        preset,
        frame_skip
    );

    let reframe_dir = std::env::current_dir()
        .unwrap_or_else(|_| Path::new(".").to_path_buf())
        .join("novaclip_reframe");
    let python_path = find_venv_python(&reframe_dir);

    // If frame_skip > 1, decimate framerate before reframe then restore after
    if frame_skip > 1 {
        let parent = input.parent().unwrap();
        let stem = input.file_stem().unwrap().to_str().unwrap();
        let decimated = parent.join(format!("{}_decimated.mp4", stem));
        let reframe_part = parent.join(format!("{}_reframe_part.mp4", stem));

        let fps = probe_fps(input).await?;
        let half_fps = (fps / frame_skip as f64).max(1.0);

        // Step 1: reduce framerate (drops frames, keeps duration)
        let decimate = tokio::process::Command::new("ffmpeg")
            .args([
                "-y", "-i",
                input.to_str().unwrap(),
                "-vf", &format!("fps={}", half_fps),
                "-an",
                decimated.to_str().unwrap(),
            ])
            .status()
            .await?;
        if !decimate.success() {
            anyhow::bail!("Frame decimation ffmpeg failed");
        }

        // Step 2: run reframe on decimated video
        let reframe_cmd = tokio::process::Command::new(&python_path)
            .args([
                "-m", "novaclip_reframe",
                decimated.to_str().unwrap(),
                reframe_part.to_str().unwrap(),
                "--preset", preset,
            ])
            .env("PYTHONPATH", reframe_dir.to_str().unwrap())
            .stderr(Stdio::piped())
            .output()
            .await?;
        if !reframe_cmd.status.success() {
            let stderr = String::from_utf8_lossy(&reframe_cmd.stderr);
            tokio::fs::remove_file(&decimated).await.ok();
            tokio::fs::remove_file(&reframe_part).await.ok();
            anyhow::bail!("novaclip-reframe failed:\n{}", stderr);
        }

        // Step 3: restore original framerate (duplicates frames, keeps duration)
        let dec_fps_str = format!("{:.2}", half_fps);
        let restore = tokio::process::Command::new("ffmpeg")
            .args([
                "-y",
                "-i", reframe_part.to_str().unwrap(),
                "-i", input.to_str().unwrap(),
                "-map", "0:v:0",
                "-map", "1:a:0",
                "-c:v", "libx264", "-preset", "fast", "-crf", "18",
                "-vf", &format!("fps={}", fps),
                "-pix_fmt", "yuv420p",
                "-c:a", "aac", "-b:a", "192k",
                "-shortest",
                output.to_str().unwrap(),
            ])
            .status()
            .await?;

        tokio::fs::remove_file(&decimated).await.ok();
        tokio::fs::remove_file(&reframe_part).await.ok();

        if !restore.success() {
            anyhow::bail!("Framerate restore ffmpeg failed");
        }

        info!("Vertical reframe complete (skip={}, {}fps->{}fps): {}", frame_skip, dec_fps_str, fps, output.display());
        return Ok(());
    }

    // No skipping — run directly
    let output_cmd = tokio::process::Command::new(&python_path)
        .args([
            "-m",
            "novaclip_reframe",
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            "--preset",
            preset,
        ])
        .env("PYTHONPATH", reframe_dir.to_str().unwrap())
        .stderr(Stdio::piped())
        .output()
        .await?;

    if !output_cmd.status.success() {
        let stderr = String::from_utf8_lossy(&output_cmd.stderr);
        anyhow::bail!("novaclip-reframe failed:\n{}", stderr);
    }

    info!("Vertical reframe complete: {}", output.display());
    Ok(())
}
