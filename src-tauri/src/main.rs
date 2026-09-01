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

fn spawn_backend(app: &tauri::AppHandle) {
    let app_data = app.path().app_data_dir().unwrap_or_else(|_| std::env::temp_dir());
    let resource_dir = app.path().resource_dir().unwrap_or_else(|_| app_data.clone());
    let db_path = app_data.join("novaclip.db");
    let output_dir = app_data.join("outputs");
    let temp_dir = app.path().app_cache_dir().unwrap_or_else(|_| app_data.join("temp"));

    let _ = std::fs::create_dir_all(&output_dir);
    let _ = std::fs::create_dir_all(&temp_dir);
    let _ = std::fs::create_dir_all(app_data.join("data"));

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

            // Attempt to spawn sidecar backend (only in bundled app; in dev it will log and continue)
            spawn_backend(&app.handle());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running NovaClip desktop");
}
