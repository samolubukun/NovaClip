# NovaClip Desktop (Tauri 2)

Flawless cross-platform bundling: **Windows (NSIS .exe + MSI)**, **macOS (DMG universal)**, **Linux (AppImage / .deb / .rpm)**. Backend runs as a sidecar on `127.0.0.1:8000`.

## Architecture
- **Frontend** (`frontend/dist`) served via `tauri://localhost` (Vite build with `VITE_API_URL=http://127.0.0.1:8000`).
- **Backend sidecar** `novaclip-api` (Axum + worker in-process, SQLite WAL) spawned by `src-tauri/src/main.rs:29` with env:
  ```
  DATABASE_URL=sqlite://<app_data>/novaclip.db?mode=rwc
  OUTPUT_DIR=<app_data>/outputs
  TEMP_DIR=<app_cache>/temp
  PORT=8000
  CORS_ORIGINS=tauri://localhost,https://tauri.localhost,http://localhost:5173
  ```
- `app_data` = `AppData/Roaming/com.novaclip.desktop` (Win) / `~/Library/Application Support/com.novaclip.desktop` (mac) / `~/.local/share/com.novaclip.desktop` (Linux).
- **FFmpeg** is required at runtime (bundled on Linux AppImage via `bundleMediaFramework`, otherwise expects `ffmpeg` in PATH). **Python reframe** (YOLO/MediaPipe/torch) is optional — backend degrades gracefully if `novaclip_reframe/venv` missing.

## Local Dev
```bash
# 1. Backend (terminal 1)
cd backend && cargo run -p novaclip-api  # http://127.0.0.1:8000

# 2. Frontend (terminal 2)
cd frontend && npm run dev               # http://localhost:5173

# 3. Tauri shell (terminal 3) — spawns sidecar automatically in bundled mode, in dev it reuses #1
npm install                              # root (tauri-cli)
npm run tauri:dev                        # or: npx tauri dev  (from repo root, config is src-tauri/tauri.conf.json)
```

`scripts/setup-desktop.mjs` builds the sidecar for your host triple and copies to `src-tauri/binaries/novaclip-api-{triple}[.exe]` (Tauri `externalBin` resolution).

```bash
node scripts/setup-desktop.mjs
npm run tauri:build                      # -> src-tauri/target/release/bundle/
```

## Production Build (CI)
GitHub workflow `.github/workflows/tauri.yml` is the source of truth:

- Matrix: `windows-latest` (x86_64-pc-windows-msvc), `ubuntu-22.04` (x86_64-unknown-linux-gnu), `macos-latest` (universal-apple-darwin via `lipo`).
- Steps: `cargo build --release -p novaclip-api` per target -> copy to `src-tauri/binaries/novaclip-api-{target}[.exe]` -> `npm run build --prefix frontend` with `VITE_API_URL=http://127.0.0.1:8000` -> `tauri-apps/tauri-action@v0.5` (`args --target universal-apple-darwin` on mac).
- Linux deps: `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libfuse2 ffmpeg`.
- Artifacts: `*.exe, *.msi, *.dmg, *.app.tar.gz, *.deb, *.AppImage, *.rpm` uploaded as `novaclip-{platform}-{target}` (14d retention).
- Release: on tag `v*` creates draft `novaclip-v__VERSION__` with updater disabled (`includeUpdaterJson: false`).

Enable auto-updater:
```bash
npm run tauri signer generate -- -w ~/.tauri/novaclip.key
# set secrets TAURI_SIGNING_PRIVATE_KEY / TAURI_SIGNING_PRIVATE_KEY_PASSWORD
# set tauri.conf.json plugins.updater.pubkey
```

## Installers
- **Windows**: `NovaClip_0.1.0_x64-setup.exe` (NSIS, currentUser) + MSI. WebView2 bootstrapper embedded (`webviewInstallMode.embedBootstrapper.silent`).
- **macOS**: `NovaClip_0.1.0_universal.dmg` (10.15+, unsigned until Apple signing). `signingIdentity: null`.
- **Linux**: `NovaClip_0.1.0_amd64.deb` + `AppImage` + `rpm`.

## Troubleshooting
- `resource path binaries/novaclip-api-... doesn't exist` -> run `node scripts/setup-desktop.mjs` or let CI build sidecar first.
- `csp null` is intentional to allow `tauri://localhost` -> `http://127.0.0.1:8000` fetches. Harden later with `connect-src`.
- Port 8000 occupied -> set `NOVACLIP_PORT` env before launch or change `PORT` in `spawn_backend`.
- FFmpeg missing -> install via `winget install ffmpeg` / `brew install ffmpeg` / `apt install ffmpeg`.
