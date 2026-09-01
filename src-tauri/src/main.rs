#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{Manager, Emitter};
use tauri_plugin_shell::ShellExt;

#[tauri::command]
fn get_backend_url() -> String {
    std::env::var("NOVACLIP_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8000".to_string())
}

#[tauri::command]
fn get_app_data_dir(app: tauri::AppHandle) -> Result<String, String> {
    app.path()
        .app_data_dir()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn check_backend_health(url: String) -> Result<bool, String> {
    let client = reqwest::Client::new();
    let health = format!("{}/health", url.trim_end_matches('/'));
    match client.get(&health).timeout(std::time::Duration::from_secs(3)).send().await {
        Ok(r) => Ok(r.status().is_success()),
        Err(e) => Err(e.to_string()),
    }
}

fn ensure_venv(app: &tauri::AppHandle) {
    // Zero-dep venv bootstrap: use bundled python embed if system python missing,
    // create app_data/novaclip_reframe/venv on first launch and pip install reframe deps.
    // This makes desktop "everything there" after first 2-min setup, installer stays ~300MB.
    let app_data = app.path().app_data_dir().unwrap_or_else(|_| std::env::temp_dir());
    let resource_dir = app.path().resource_dir().unwrap_or_else(|_| app_data.clone());
    let venv_dir = app_data.join("novaclip_reframe").join("venv");
    let venv_python = if cfg!(windows) {
        venv_dir.join("Scripts").join("python.exe")
    } else {
        venv_dir.join("bin").join("python")
    };
    if venv_python.exists() {
        tracing::info!("Venv already ready: {:?}", venv_python);
        return;
    }
    // Find python to bootstrap: prefer bundled embed, then system python
    let bundled_python = {
        let candidates = [
            resource_dir.join("python").join("python.exe"),
            resource_dir.join("python").join("bin").join("python3"),
            resource_dir.join("python").join("python"),
            resource_dir.join("resources").join("python").join("python.exe"),
        ];
        candidates.into_iter().find(|p| p.exists())
    };
    let system_python = ["python3", "python"]
        .iter()
        .map(|bin| {
            if cfg!(windows) {
                // where python
                std::process::Command::new("where").arg(bin).output()
                    .ok().and_then(|o| String::from_utf8(o.stdout).ok())
                    .and_then(|s| s.lines().next().map(|l| std::path::PathBuf::from(l.trim())))
            } else {
                std::process::Command::new("which").arg(bin).output()
                    .ok().and_then(|o| String::from_utf8(o.stdout).ok())
                    .map(|s| std::path::PathBuf::from(s.trim()))
                    .filter(|p| p.exists())
            }
        })
        .find(|p| p.is_some()).flatten();

    let bootstrap = bundled_python.or(system_python);
    let Some(py) = bootstrap else {
        tracing::warn!("No python found for venv bootstrap (bundled nor system) — reframe will be disabled until python available");
        return;
    };
    tracing::info!("Bootstrapping venv with {:?} -> {:?}", py, venv_dir);
    let _ = std::fs::create_dir_all(venv_dir.parent().unwrap());
    let out = std::process::Command::new(&py).arg("-m").arg("venv").arg(&venv_dir).output();
    match out {
        Ok(o) if o.status.success() => tracing::info!("venv created"),
        Ok(o) => {
            tracing::warn!("venv create failed: {}", String::from_utf8_lossy(&o.stderr));
            return;
        }
        Err(e) => {
            tracing::warn!("venv spawn failed: {}", e);
            return;
        }
    }
    // Ensure pip and install reframe deps (CPU torch to keep lite)
    let pip = if cfg!(windows) { venv_dir.join("Scripts").join("pip.exe") } else { venv_dir.join("bin").join("pip") };
    let reqs = [
        "yt-dlp", "edge-tts", "gtts",
        "ultralytics>=8.3.0", "mediapipe==0.10.14", "numpy>=1.26",
        "opencv-python>=4.10.0.84", "scenedetect>=0.6.4", "lap>=0.5.12",
        "torch>=2.2", "torchvision>=0.17",
    ];
    let mut cmd = std::process::Command::new(&pip);
    cmd.arg("install").arg("--no-cache-dir").arg("--extra-index-url").arg("https://download.pytorch.org/whl/cpu");
    for r in reqs { cmd.arg(r); }
    // Run in background thread so UI not blocked
    std::thread::spawn(move || {
        tracing::info!("Installing reframe deps into venv (first launch, ~2min)...");
        match cmd.output() {
            Ok(o) if o.status.success() => tracing::info!("venv deps installed"),
            Ok(o) => tracing::warn!("pip install failed: {}", String::from_utf8_lossy(&o.stderr)),
            Err(e) => tracing::warn!("pip spawn failed: {}", e),
        }
    });
}

fn spawn_backend(app: &tauri::AppHandle) {
    let app_data = app.path().app_data_dir().unwrap_or_else(|_| std::env::temp_dir());
    let resource_dir = app.path().resource_dir().unwrap_or_else(|_| app_data.clone());
    let db_path = app_data.join("novaclip.db");
    let output_dir = app_data.join("outputs");
    let temp_dir = app.path().app_cache_dir().unwrap_or_else(|_| app_data.join("temp"));

    let _ = std::fs::create_dir_all(&output_dir);
    let _ = std::fs::create_dir_all(&temp_dir);
    let _ = std::fs::create_dir_all(app_data.join("data"));
    // Also ensure bundled ffmpeg is on PATH for sidecar
    if let Ok(res) = app.path().resource_dir() {
        let ffmpeg_candidates = [
            res.join("ffmpeg"), res.join("ffmpeg.exe"),
            res.join("bin").join("ffmpeg"), res.join("bin").join("ffmpeg.exe"),
        ];
        if let Some(ff) = ffmpeg_candidates.iter().find(|p| p.exists()) {
            if let Some(dir) = ff.parent() {
                let cur = std::env::var("PATH").unwrap_or_default();
                let sep = if cfg!(windows) { ";" } else { ":" };
                let new_path = format!("{}{}{}", dir.display(), sep, cur);
                // SAFETY: single-threaded setup
                unsafe { std::env::set_var("PATH", new_path); }
                tracing::info!("Added bundled ffmpeg to PATH: {:?}", ff);
            }
        }
        // Ensure venv bin is on PATH for sidecar python reframe
        let venv_bin = app_data.join("novaclip_reframe").join("venv").join(if cfg!(windows) { "Scripts" } else { "bin" });
        if venv_bin.exists() {
            let cur = std::env::var("PATH").unwrap_or_default();
            let sep = if cfg!(windows) { ";" } else { ":" };
            let new_path = format!("{}{}{}", venv_bin.display(), sep, cur);
            unsafe { std::env::set_var("PATH", new_path); }
        }
    }

    // DATABASE_URL needs sqlite:// prefix with absolute path
    let db_url = format!("sqlite://{}?mode=rwc", db_path.to_string_lossy().replace('\\', "/"));
    let port = std::env::var("NOVACLIP_PORT").unwrap_or_else(|_| "8000".to_string());

    tracing::info!("Spawning novaclip-api sidecar: db_url={}, output={:?}, temp={:?}, port={}", db_url, output_dir, temp_dir, port);
    tracing::info!("Resource dir: {:?}", resource_dir);

    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let sidecar = match app_handle.shell().sidecar("novaclip-api") {
            Ok(s) => s
                .env("DATABASE_URL", db_url)
                .env("OUTPUT_DIR", output_dir.to_string_lossy().to_string())
                .env("TEMP_DIR", temp_dir.to_string_lossy().to_string())
                .env("PORT", port)
                .env("CORS_ORIGINS", "http://localhost:5173,http://localhost:3000,tauri://localhost,https://tauri.localhost,http://tauri.localhost")
                .env("RUST_LOG", "novaclip_api=info,novaclip_worker=info"),
            Err(e) => {
                tracing::warn!("Sidecar binary not found (dev mode - run backend manually): {}", e);
                return;
            }
        };

        match sidecar.spawn() {
            Ok((mut rx, child)) => {
                tracing::info!("Sidecar spawned, pid: {:?}", child.pid());
                use tauri_plugin_shell::process::CommandEvent;
                while let Some(event) = rx.recv().await {
                    match event {
                        CommandEvent::Stdout(line) => tracing::info!("[backend] {}", String::from_utf8_lossy(&line)),
                        CommandEvent::Stderr(line) => tracing::warn!("[backend] {}", String::from_utf8_lossy(&line)),
                        CommandEvent::Terminated(payload) => {
                            tracing::warn!("Backend terminated: {:?}", payload);
                            app_handle.emit("backend-terminated", format!("{:?}", payload)).ok();
                            break;
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to spawn sidecar: {}", e);
                app_handle.emit("backend-error", format!("spawn failed: {}", e)).ok();
            }
        }
    });
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_backend_url, get_app_data_dir, check_backend_health])
        .setup(|app| {
            // Logging
            #[cfg(debug_assertions)]
            {
                let _ = tracing_subscriber::fmt::try_init();
            }

            // Ensure app data dirs exist for backend: outputs, temp, data
            if let Ok(app_data) = app.path().app_data_dir() {
                let _ = std::fs::create_dir_all(app_data.join("outputs"));
                let _ = std::fs::create_dir_all(app_data.join("temp"));
                let _ = std::fs::create_dir_all(app_data.join("data"));
                tracing::info!("App data dir: {:?}", app_data);
            }
            if let Ok(app_cache) = app.path().app_cache_dir() {
                let _ = std::fs::create_dir_all(app_cache.join("temp"));
            }

            // Zero-dep venv bootstrap + bundled ffmpeg PATH, then spawn sidecar
            ensure_venv(&app.handle());
            spawn_backend(&app.handle());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running NovaClip desktop");
}
