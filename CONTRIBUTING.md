# Contributing to NovaClip

First off - thank you for being here. NovaClip is an ambitious, AI-heavy project
that spans Rust video pipelines, Python CV, React, and a growing list of
third-party AI integrations. **We need your help.**

Many of the pipelines (Nova Studio AI Shorts, YouTube Studio tools, Nova Edit's
agent workflow, MCP tools, and the social-posting integration) are **prototype
quality**. They work, but they are far from battle-tested. Bugs, missing
fallbacks, brittle API integrations, and rough UX are expected - and fixing them
is exactly the kind of contribution that matters most.

---

## Table of Contents

- [Getting Started](#getting-started)
- [Project Layout](#project-layout)
- [How to Contribute](#how-to-contribute)
- [Development Workflow](#development-workflow)
- [Coding Conventions](#coding-conventions)
- [Testing](#testing)
- [Where Help Is Most Needed](#where-help-is-most-needed)
- [Code of Conduct](#code-of-conduct)
- [License](#license)

---

## Getting Started

Prerequisites:

- **Rust** toolchain (`cargo`, `rustc` 1.80+)
- **Node.js** (v18+) and `npm`
- **FFmpeg** in PATH
- **yt-dlp** in PATH or at `backend/yt-dlp.exe`
- **Python 3.11+** (required for AI vertical reframe / subject tracking)

### 1. Backend

```bash
cd backend

# Python venv for the AI vertical-reframe engine (YOLO + MediaPipe)
python -m venv novaclip_reframe/venv
novaclip_reframe/venv\Scripts\pip install --extra-index-url https://download.pytorch.org/whl/cpu \
    ultralytics>=8.3.0 mediapipe==0.10.14 opencv-python>=4.10.0.84 \
    scenedetect>=0.6.4 lap>=0.5.12

# Run migrations & start the Axum API server on :8000
cargo run --bin novaclip-api
```

### 2. Frontend

```bash
cd frontend
npm install
npm run dev        # Vite dev server on :5173
```

### 3. API Keys

NovaClip is 100% BYOK (bring your own key). Most features need at least a
Gemini key. See the [BYOK section of the README](README.md#-byok-bring-your-own-keys)
for the full list and where to get keys.

---

## Project Layout

```
NovaClip/
├── backend/
│   ├── crates/
│   │   ├── api/          # Axum HTTP routes, SSE progress, MCP server
│   │   ├── db/           # SQLite schema & SQLx queries
│   │   └── worker/       # The video processing pipeline
│   │       └── src/pipeline/   # clip, caption, reframe, tts, scraper,
│   │                          # studio_llm, wavespeed, nova_edit, repurpose, ...
│   ├── novaclip_reframe/ # Python package: YOLO + MediaPipe subject tracking
│   ├── migrations/       # SQLite migrations
│   └── Cargo.toml
├── frontend/
│   └── src/
│       ├── pages/        # Home (Clipper), Studio, NovaEdit, Repurpose,
│       │                 # YouTubeStudio, Task, History
│       ├── components/   # Nav, SettingsModal, shared UI
│       └── lib/          # API client, SSE listeners, model lists
├── docker-compose.yml
├── Makefile
└── README.md
```

---

## How to Contribute

### Report a bug

Open an issue and include:

- The mode/feature involved (Clipper / Studio · Stock / AI / AI Shorts / Nova
  Edit / Repurpose / YouTube Studio / MCP / publish).
- Steps to reproduce, expected vs. actual behavior.
- Browser + OS, and whether the backend is running natively or in Docker.
- Any relevant backend logs (stderr) or frontend console errors.

### Request a feature / propose an improvement

The [Roadmap](README.md#-roadmap) is a good place to start. Modes, pipelines,
workflows, integrations, and models are all open territory. Open an issue to
discuss before starting large work.

### Submit a pull request

1. Fork the repo and create a feature branch.
2. Make your change. Keep PRs focused and small.
3. Make sure the project still compiles (see [Testing](#testing)).
4. Open a PR against `main` with a clear description of what and why.
5. Reference any related issue.

---

## Development Workflow

- **Branch naming**: `feat/`, `fix/`, `refactor/`, `docs/`, `chore/` prefixes,
  e.g. `feat/studio-voice-cloning`.
- **Commits**: small, atomic commits with a concise message. Follow the style of
  existing history.
- **Target**: one feature or fix per PR.

---

## Coding Conventions

- **Rust**: `cargo fmt` and `cargo clippy` before pushing. No `unwrap()` on
  user/network data - use `anyhow`, `Result`, and graceful fallbacks. Async code
  goes through Tokio.
- **TypeScript/React**: the project is fully typed - no `any` unless truly
  necessary. Follow existing component patterns (inline styles, lucide icons,
  `framer-motion`, `sonner` toasts).
- **Fail loudly, fall back gracefully**: pipelines should degrade (e.g. Pexels
  fallback when WaveSpeed fails) rather than crash.
- **No secrets**: never commit API keys. Keys live in browser
  localStorage (`novaclip_*`) or `.env` / env vars.

---

## Testing

There is no formal test suite yet - **a great place to contribute**.

At minimum, before opening a PR:

```bash
# Backend compiles
cd backend && cargo check -p novaclip-api -p novaclip-worker

# Frontend type-checks
cd frontend && npx tsc -b
```

If you change a pipeline, verify end-to-end with a real task (Clipper URL,
Studio script, Nova Edit upload, etc.) and note in the PR what you tested.

---

## Where Help Is Most Needed

Contributions are welcome across **every part of NovaClip** - the forms,
workflows, UI/UX, backend, integrations, performance, and docs. Every mode and
area can be improved:

- **Nova Clipper** - transcription accuracy, scoring quality, cropping/smart
  reframe, caption animations, chat-based clip editing, export options.
- **Nova Studio - Stock B-Roll** - Pexels/Pixabay/Pinterest search quality,
  keyword matching, clip trimming, video/photo mixing.
- **Nova Studio - AI B-Roll** - WaveSpeed Seedance reliability, Pexels fallbacks,
  Lyria AI music, parallel rendering.
- **AI Shorts pipeline** (WaveSpeed actor/lip-sync, Seedance B-roll, TTS,
  premium InfiniteTalk hosting) - resilience, fallbacks, cost controls.
- **YouTube Studio tools** - thumbnail image-model selection, title/description
  quality, edge cases for videos/transcripts.
- **Nova Edit agent workflow** - director/editor/reviewer reliability, better
  shot selection, transitions, color grading, music.
- **Nova Repurpose** - more platforms, better written-content copy, campaign
  reporting.
- **History / Task pages** - task filtering, status clarity, error surfacing,
  retry/resume UX, download flows.
- **MCP server** - new tools, richer schemas, better error responses.
- **Social publishing** (Upload-Post) - more platforms, retry logic, metadata.
- **AI models & integrations** - swap-in providers, local/offline models
  (Ollama, local STT), new WaveSpeed/Gemini models, caching.
- **Forms & UX** - validation, defaults, responsive layouts, accessibility,
  keyboard shortcuts, and error messages.
- **Performance** - parallelization, queueing, FFmpeg efficiency, memory.
- **Packaging** - the **Tauri desktop build** is on the roadmap; native setup,
  auto-updates, and offline-first packaging are wide open.
- **Tests & CI** - a test suite and a CI pipeline would help everyone.

---

## Code of Conduct

Please read and follow our [Code of Conduct](CODE_OF_CONDUCT.md). Be kind. This
project welcomes everyone regardless of experience level. Harassment of any kind
is not tolerated. Disagreements happen - keep them technical and respectful.

## Reporting Security Vulnerabilities

Please do **not** report security issues publicly. Instead, use the
[Security Policy](SECURITY.md) for private reporting.

---

## License

By contributing you agree that your contributions are licensed under the same
[MIT License](LICENSE) as the project.
