<div align="center">

<img src="frontend/public/logo.jpg" alt="NovaClip Logo" width="160" style="border-radius: 20px; box-shadow: 0 10px 30px rgba(0,0,0,0.5);" />

# **NovaClip**

### **AI-Powered Viral Video Clip Generator & Faceless AI Creator Engine**

Turn long podcasts, YouTube videos, and live streams into high-converting vertical short clips — OR generate complete faceless AI videos from scripts/topics with automated stock media scraping, multi-provider neural voiceovers, custom typography, brand watermarks, and multi-color animated karaoke captions.

[![Rust](https://img.shields.io/badge/Rust-CE412B?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Axum](https://img.shields.io/badge/Axum-000000?style=for-the-badge&logo=rust&logoColor=orange)](https://github.com/tokio-rs/axum)
[![React](https://img.shields.io/badge/React_19-20232A?style=for-the-badge&logo=react&logoColor=61DAFB)](https://react.dev/)
[![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?style=for-the-badge&logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![Vite](https://img.shields.io/badge/Vite-646CFF?style=for-the-badge&logo=vite&logoColor=white)](https://vitejs.dev/)
[![Gemini AI](https://img.shields.io/badge/Gemini_3.1-8E75B2?style=for-the-badge&logo=googlegemini&logoColor=white)](https://ai.google.dev/)
[![OpenRouter](https://img.shields.io/badge/OpenRouter_Free_Models-6566F1?style=for-the-badge&logo=openai&logoColor=white)](https://openrouter.ai/)
[![Deepgram](https://img.shields.io/badge/Deepgram_Nova--3-13EF95?style=for-the-badge&logo=deepgram&logoColor=black)](https://deepgram.com/)
[![FFmpeg](https://img.shields.io/badge/FFmpeg-007808?style=for-the-badge&logo=ffmpeg&logoColor=white)](https://ffmpeg.org/)
[![SQLite](https://img.shields.io/badge/SQLite-003B57?style=for-the-badge&logo=sqlite&logoColor=white)](https://www.sqlite.org/)
[![Docker](https://img.shields.io/badge/Docker-2496ED?style=for-the-badge&logo=docker&logoColor=white)](https://www.docker.com/)

</div>

---

## Features

- **Nova Studio — Faceless AI Creator**: Generate complete faceless videos from scratch. Single continuous voiceover (never segmented), full word-level timestamp alignment, global ASS karaoke captions synced exactly to audio, and media clips trimmed-to-duration concatenated into one seamless video.
- **AI Brains (Gemini 3.1 Flash-Lite & OpenRouter)**: Script generation and virality analysis powered by Gemini 3.1 Flash-Lite (Default), Gemini 3.1 Pro, or OpenRouter free models (`meta-llama/llama-3.3-70b-instruct:free`, `deepseek/deepseek-r1:free`, `qwen/qwen-2.5-72b-instruct:free`). Topic-to-script generation targets exact word count for selected duration.
- **Multi-Source Stock Media Scrapers**: Automated HD video and photo scraping with multi-platform fallback across **Pinterest Video & Photo Scraper** (Playwright + yt-dlp), **Pexels API**, and **Pixabay API**.
- **Multi-Provider Neural Voiceovers (TTS) with Duration Sync**: Choose between **Microsoft Edge-TTS** (free, 10+ languages, estimated word timestamps), **ElevenLabs API** (native /with-timestamps endpoint for character-level alignment), or **Deepgram Aura TTS** + Deepgram STT (model nova-2) for precise word timing. Final video uses `-shortest` to ensure audio and video always end together.
- **AI Virality Scoring**: Gemini AI evaluates hooks, retention probability, emotional peaks, and shareability for every extracted segment.
- **Global ASS Karaoke Subtitles**: Word-level timestamps from any TTS provider drive multi-color active word-pop subtitle animations. Generated as a single `.ass` file for the entire video with per-word timing, no timestamp rebasing needed.
- **Font & Typography Library**: Full studio typography control with 12 popular fonts (The Bold Font, TikTok Sans, Montserrat, Impact, Bebas Neue, Inter, etc.) and custom font size slider.
- **AI Caption Color Palette Picker**: Customize primary text color and highlight accent color per clip with real-time sync.
- **Brand Watermark Overlay**: Upload transparent PNG/WEBP/JPEG logos with configurable positioning (Top Right, Top Left, Bottom Left, Bottom Right) and opacity slider. Looped across full video duration via ffmpeg `-loop 1` with per-frame alpha blending.
- **Preset Caption Animations**: Choose between Bouncy Word-by-Word Pop, Typewriter Reveal, Smooth Fade-In, and Slide Up.
- **NLP Emoji Auto-Insertion**: Automatically analyze transcript sentiment and append contextually relevant emojis to key caption words.
- **Multi-Provider Speech-to-Text (STT)**: Choose between **Deepgram Nova-3** (cloud, fast word-level timestamps & multi-speaker diarization), **Vosk Local** (fast offline CPU speech recognition), or **Whisper Local** (`ggml-base.bin`, high-accuracy offline neural transcription).
- **AI Vertical Reframe & Split-Screen**: YOLO + MediaPipe subject tracking with selectable camera modes (Single, Split Screen, Auto) and Deepgram v2 active-speaker diarization / hard cuts. Custom divider line and preset controls.
- **Live Device Mockup Preview**: Dynamic aspect ratio live preview showing realistic frames for Vertical 9:16 (mobile notch frame), Square 1:1, Widescreen 16:9, and Original video.
- **Originality Boost**: Apply brightness, contrast, and saturation adjustments to alter perceptual video hashes.
- **AI Caption Translation**: Translate captions to over 22 languages (Spanish, French, German, Japanese, Korean, Chinese, Portuguese, Italian, Arabic, etc.) via Gemini.
- **AI Edit Chat**: Natural-language editing commands on generated clips (trim, delete, reframe, captions, translate, memes, merge) via a floating chat panel on the Task page.
- **Smart Multi-Aspect Cropping**: Auto-detects and centers active speakers for 9:16 vertical shorts, 1:1 square posts, or 16:9 widescreen clips.
- **Zip Export**: Export individual clips or download all generated clips in a single `.zip` package.
- **100% BYOK (Bring Your Own Key)**: Local browser storage for Gemini, OpenRouter, ElevenLabs, Deepgram, Pexels, and Pixabay keys. Runs 100% offline-compatible for desktop & local deployment.

---

## Technology Stack

| Layer | Technologies & Tools |
|---|---|
| **Backend API** | Rust, Axum 0.8, Tokio Async Runtime, SQLx SQLite (WAL mode) |
| **Video Engine** | FFmpeg, yt-dlp, Tokio MPSC In-Process Async Queue, Playwright Chromium (Pinterest scraper) |
| **Subject Tracking** | Python 3.11, Ultralytics YOLO11n-seg, MediaPipe Face/Pose, OpenCV, SceneDetect |
| **Speech AI / STT / TTS** | Deepgram Nova-3 STT, Vosk Local STT, Whisper Local (ggml-base.bin), ElevenLabs with-timestamps API, Edge-TTS, Deepgram Aura TTS |
| **Analysis & LLM AI** | Google Gemini AI Virality Analysis, Caption Translation, Edit Reasoning |
| **Frontend UI** | React 19, TypeScript, Vite 6, Framer Motion, Lucide Icons |
| **DevOps & Packaging** | Docker, Docker Compose, Nginx, Makefile |

---

## Quick Start & Installation

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
- **Python 3.11+** with virtual environment (required for AI vertical reframe)

#### 1. Backend Setup:
```bash
cd backend
# Set up Python venv for AI vertical reframe
python -m venv novaclip_reframe/venv
novaclip_reframe/venv\Scripts\pip install --extra-index-url https://download.pytorch.org/whl/cpu \
    ultralytics>=8.3.0 mediapipe==0.10.14 opencv-python>=4.10.0.84 \
    scenedetect>=0.6.4 lap>=0.5.12

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

---

## License

Distributed under the MIT License. See `LICENSE` for more information.

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
│   │   ├── api/          # Axum HTTP routes & SSE progress streaming
│   │   │   └── src/routes/ (tasks, ai_edit, media, etc.)
│   │   ├── db/           # SQLite database models & SQLx queries
│   │   └── worker/       # Video processing pipeline
│   │       └── src/pipeline/ (clip, caption, reframe, originality, translate, tts, scraper, studio_llm, ...)
│   ├── novaclip_reframe/ # Python package for AI vertical reframe (YOLO + MediaPipe)
│   │   ├── novaclip_reframe/
│   │   │   └── auto_reframe.py  # Core subject-tracking engine
│   │   └── venv/         # Python virtual environment (gitignored)
│   ├── migrations/       # Database SQL schema migrations
│   └── Cargo.toml        # Rust workspace root configuration
├── frontend/
│   ├── src/
│   │   ├── components/   # Reusable UI components (Nav, SettingsModal)
│   │   ├── pages/        # Home, Task, History page views
│   │   └── lib/          # API client & SSE progress listeners
│   └── package.json      # Frontend React + Vite dependencies
├── docker-compose.yml    # Multi-container orchestration
└── README.md             # Project documentation
```

---

## 📜 License

MIT License © [Samuel Olubukun](https://github.com/samolubukun)
