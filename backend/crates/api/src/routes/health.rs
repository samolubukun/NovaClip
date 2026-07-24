use axum::{routing::get, Router, Json};
use serde_json::json;
use crate::state::AppState;

pub fn health_router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_check))
        .route("/", get(root))
}

async fn root() -> Json<serde_json::Value> {
    Json(json!({
        "name": "NovaClip API",
        "version": "0.1.0",
        "docs": "/health"
    }))
}

async fn health_check(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Json<serde_json::Value> {
    let db_ok = sqlx::query("SELECT 1")
        .execute(&state.db)
        .await
        .is_ok();
    Json(json!({
        "status": if db_ok { "healthy" } else { "degraded" },
        "database": if db_ok { "connected" } else { "disconnected" }
    }))
}
