# Multi-stage Rust build
FROM rust:1.83-slim AS builder

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev libpq-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY Cargo.toml ./
COPY crates ./crates
COPY migrations ./migrations

# Build the API binary (it runs the worker pipeline in-process)
RUN cargo build --release

# API runtime (runs the full pipeline in-process, including Python AI reframe)
FROM debian:bookworm-slim AS api
RUN apt-get update && apt-get install -y \
    libssl3 libpq5 ca-certificates ffmpeg python3 python3-pip python3-venv \
    fonts-dejavu-core fonts-noto-color-emoji \
    libgl1 libglib2.0-0 libgomp1 \
    && rm -rf /var/lib/apt/lists/* && \
    python3 -m venv /app/novaclip_reframe/venv && \
    /app/novaclip_reframe/venv/bin/pip install --no-cache-dir \
        yt-dlp \
        edge-tts gtts \
        ultralytics>=8.3.0 \
        mediapipe==0.10.14 \
        numpy>=1.26 \
        opencv-python>=4.10.0.84 \
        scenedetect>=0.6.4 \
        lap>=0.5.12 \
        torch>=2.2 \
        torchvision>=0.17
WORKDIR /app
COPY --from=builder /build/target/release/novaclip-api ./
COPY --from=builder /build/migrations ./migrations
COPY pinterest_scraper.py edge_tts_runner.py ./
COPY novaclip_reframe ./novaclip_reframe
COPY models/vosk-model-small-en-us-0.15 ./models/vosk-model-small-en-us-0.15
RUN mkdir -p outputs temp/uploads
ENV PATH="/app/novaclip_reframe/venv/bin:${PATH}"
EXPOSE 8000
CMD ["./novaclip-api"]

# Worker runtime (kept for standalone use / a future compose worker service)
FROM debian:bookworm-slim AS worker
RUN apt-get update && apt-get install -y \
    libssl3 libpq5 ca-certificates ffmpeg python3 python3-pip python3-venv fonts-noto-color-emoji \
    && rm -rf /var/lib/apt/lists/* && \
    python3 -m venv /app/venv && \
    /app/venv/bin/pip install --no-cache-dir \
        yt-dlp \
        edge-tts gtts \
        ultralytics>=8.3.0 \
        mediapipe==0.10.14 \
        numpy>=1.26 \
        opencv-python>=4.10.0.84 \
        scenedetect>=0.6.4 \
        lap>=0.5.12 \
        torch>=2.2 \
        torchvision>=0.17
WORKDIR /app
COPY --from=builder /build/target/release/novaclip-worker ./
COPY novaclip_reframe ./novaclip_reframe
COPY pinterest_scraper.py edge_tts_runner.py ./
RUN mkdir -p outputs temp/uploads
ENV PATH="/app/venv/bin:${PATH}"
CMD ["./novaclip-worker"]
