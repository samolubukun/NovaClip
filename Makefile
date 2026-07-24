.PHONY: dev build dev-frontend dev-backend up down logs

# Docker full stack
up:
	docker-compose up -d --build

down:
	docker-compose down

logs:
	docker-compose logs -f

# Local dev
dev-backend:
	cd backend && cargo run --bin novaclip-api

dev-worker:
	cd backend && cargo run --bin novaclip-worker

dev-frontend:
	cd frontend && npm run dev

# Build
build-backend:
	cd backend && cargo build --release

build-frontend:
	cd frontend && npm run build

# DB
migrate:
	cd backend && sqlx migrate run

# Setup
setup:
	cp .env.example .env
	@echo "Edit .env with your API keys, then run: make up"
