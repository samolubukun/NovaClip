<div align="center">

# ⚡ NovaClip

**AI-Powered Viral Video Clip Generator & Automation Engine**

Turn long podcasts, YouTube videos, and live streams into high-converting vertical short clips with AI virality scoring and animated multi-color karaoke captions.

[![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Axum](https://img.shields.io/badge/Axum-0.8-blue?style=for-the-badge&logo=rust)](https://github.com/tokio-rs/axum)
[![React](https://img.shields.io/badge/React-19-61DAFB?style=for-the-badge&logo=react&logoColor=black)](https://react.dev/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.0-3178C6?style=for-the-badge&logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![Vite](https://img.shields.io/badge/Vite-6.0-646CFF?style=for-the-badge&logo=vite&logoColor=white)](https://vitejs.dev/)
[![SQLite](https://img.shields.io/badge/SQLite-WAL_Mode-003B57?style=for-the-badge&logo=sqlite&logoColor=white)](https://www.sqlite.org/)
[![Docker](https://img.shields.io/badge/Docker-Containerized-2496ED?style=for-the-badge&logo=docker&logoColor=white)](https://www.docker.com/)

</div>

---

## ✨ Features

- 🎯 **AI Virality Scoring**: Gemini AI evaluates hooks, retention probability, emotional peaks, and shareability for every extracted segment.
- 🎙️ **Nova-3 Karaoke Subtitles**: Deepgram word-level timestamps drive multi-color active word-pop subtitle animations.
- 📱 **Smart Multi-Aspect Cropping**: Auto-detects and centers active speakers for 9:16 vertical shorts, 1:1 square posts, or 16:9 widescreen clips.
- 📦 **Zip Export**: Export individual clips or download all generated clips in a single `.zip` package.
- ⚙️ **100% BYOK (Bring Your Own Key)**: Local browser storage for Gemini & Deepgram keys. Runs 100% offline-compatible for desktop & local deployment.
- 🚀 **High-Tech Visual Pipeline**: Real-time 5-stage SSE progress stepper tracking stream downloads, speech recognition, scoring, cropping, and rendering.

---

## 🛠️ Technology Stack

| Layer | Technologies & Tools |
|---|---|
| **Backend API** | Rust, Axum 0.8, Tokio Async Runtime, SQLx SQLite (WAL mode) |
| **Video Engine** | FFmpeg, `yt-dlp`, Tokio MPSC In-Process Async Queue |
| **Speech AI** | Deepgram Nova-3 Word-Level Timestamp Alignment |
| **Analysis AI** | Google Gemini AI Virality & Retention Analysis |
| **Frontend UI** | React 19, TypeScript, Vite 6, Framer Motion, Lucide Icons |
| **DevOps & Packaging** | Docker, Docker Compose, Nginx, Makefile, Tauri Ready |

---

## 🚀 Quick Start & Installation

### Option A: Running with Docker (Recommended)

1. **Clone the Repository**:
   ```bash
   git clone https://github.com/samolubukun/NovaClip.git
   cd NovaClip
   ```

2. **Launch Container Services**:
   ```bash
   docker-compose up -d --build
   ```

3. **Access the App**:
   - **Frontend App**: `http://localhost:3000`
   - **Backend API**: `http://localhost:8000`

---

### Option B: Running Without Docker (Native Local Setup)

#### Prerequisites:
- **Rust toolchain** (`cargo`, `rustc` 1.80+)
- **Node.js** (v18+ and `npm`)
- **FFmpeg** installed system-wide and available in PATH
- **yt-dlp** installed system-wide or placed in `backend/yt-dlp.exe`

#### 1. Backend Setup:
```bash
cd backend
# Run database migrations & start Axum API server
cargo run --bin novaclip-api
```

#### 2. Frontend Setup:
```bash
cd frontend
# Install dependencies
npm install

# Start Vite dev server
npm run dev
```

Open `http://localhost:5173` in your web browser.

---

## 🔑 BYOK (Bring Your Own Keys)

NovaClip is engineered as a zero-vendor-lockin application. Configure your API keys directly inside the in-app **Settings Modal (⚙️)**:

- **Google Gemini API Key**: [Get Key at Google AI Studio](https://ai.google.dev)
- **Deepgram API Key**: [Get Key at Deepgram Console](https://console.deepgram.com)

---

## 📁 Repository Structure

```
NovaClip/
├── backend/
│   ├── crates/
│   │   ├── api/       # Axum HTTP routes & SSE progress streaming
│   │   ├── db/        # SQLite database models & SQLx queries
│   │   └── worker/    # Video processing pipeline (download, crop, caption)
│   ├── migrations/    # Database SQL schema migrations
│   └── Cargo.toml     # Rust workspace root configuration
├── frontend/
│   ├── src/
│   │   ├── components/# Reusable UI components (Nav, SettingsModal)
│   │   ├── pages/     # Home, Task, History page views
│   │   └── lib/       # API client & SSE progress listeners
│   └── package.json   # Frontend React + Vite dependencies
├── docker-compose.yml # Multi-container orchestration
└── README.md          # Project documentation
```

---

## 📜 License

MIT License © [Samuel Olubukun](https://github.com/samolubukun)
