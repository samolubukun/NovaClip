# Multi-stage Rust build
FROM rust:1.83-slim AS builder

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev libpq-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY Cargo.toml ./
COPY crates ./crates
COPY migrations ./migrations

RUN cargo build --release

# API runtime
FROM debian:bookworm-slim AS api
RUN apt-get update && apt-get install -y \
    libssl3 libpq5 ca-certificates ffmpeg \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /build/target/release/novaclip-api ./
COPY --from=builder /build/migrations ./migrations
RUN mkdir -p outputs temp/uploads
EXPOSE 8000
CMD ["./novaclip-api"]

# Worker runtime
FROM debian:bookworm-slim AS worker
RUN apt-get update && apt-get install -y \
    libssl3 libpq5 ca-certificates ffmpeg python3 python3-pip \
    && pip3 install yt-dlp --break-system-packages \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /build/target/release/novaclip-worker ./
RUN mkdir -p outputs temp/uploads
CMD ["./novaclip-worker"]
