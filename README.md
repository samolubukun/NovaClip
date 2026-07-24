# NovaClip

> AI-powered viral clip generator. Rust Axum + Vite React. Open access, no auth.

## Stack
- **Backend**: Rust + Axum + sqlx (Postgres) + Redis
- **Worker**: Tokio async pipeline — yt-dlp, FFmpeg, Deepgram Nova-3, Gemini 2.5 Flash Lite
- **Frontend**: Vite + React + TypeScript — black/yellow dark theme

## Features
- 107 features across 9 categories (see implementation_plan.md)
- Virality-aware AI highlight selection (hooks, emotional peaks, opinion bombs, quotable lines)
- Score + Hook + Reason for every clip
- Deepgram Nova-3 word-level transcription
- Long-video chunking (30-min chunks, 2-min overlap)
- Smart deduplication
- 9:16/1:1/16:9 smart crop
- 7 caption templates with karaoke animations
- Real-time SSE progress
- Clip-ready streaming
- Trim, split, merge, cancel, resume

## Quick Start

### 1. Setup
```bash
cp .env.example .env
# Edit .env: add GEMINI_API_KEY, DEEPGRAM_API_KEY
```

### 2. Run with Docker
```bash
docker-compose up -d --build
# API: http://localhost:8000
# Frontend: http://localhost:3000
```

### 3. Local dev
```bash
# Terminal 1: API
cd backend && cargo run --bin novaclip-api

# Terminal 2: Worker
cd backend && cargo run --bin novaclip-worker

# Terminal 3: Frontend
cd frontend && npm run dev
```

## API Keys
| Key | Where to get |
|-----|------|
| `GEMINI_API_KEY` | https://aistudio.google.com/apikey |
| `DEEPGRAM_API_KEY` | https://console.deepgram.com |
| `PEXELS_API_KEY` | https://www.pexels.com/api/ (optional, for B-roll) |

## Model
Gemini: `gemini-3.1-flash-lite` (configurable via `GEMINI_MODEL` env)
