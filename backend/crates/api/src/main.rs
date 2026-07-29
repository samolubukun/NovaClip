mod config;
mod state;
mod routes;

use axum::Router;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use novaclip_db::pool::create_pool;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "novaclip_api=info,novaclip_worker=info,tower_http=info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cfg = config::Config::from_env()?;

    tracing::info!("Connecting to SQLite: {}", cfg.database_url);
    let db = create_pool(&cfg.database_url).await?;

    // Run migrations at startup
    tracing::info!("Running migrations...");
    sqlx::migrate!("../../migrations")
        .run(&db)
        .await
        .expect("Failed to run migrations");

    // Create the in-process job queue channel (replaces Redis)
    let (job_tx, job_rx) = tokio::sync::mpsc::channel::<uuid::Uuid>(256);

    // Spawn the worker loop inside the API process — no separate binary needed
    let db_for_worker = db.clone();
    tokio::spawn(async move {
        novaclip_worker::run_worker(db_for_worker, job_rx).await;
    });
    tracing::info!("In-process worker started");

    let state = state::AppState {
        db,
        job_tx,
        config: cfg.clone(),
    };

    // CORS
    let origins: Vec<axum::http::HeaderValue> = cfg.cors_origins.iter()
        .filter_map(|o| o.parse().ok())
        .collect();
    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .merge(routes::health_router())
        .merge(routes::tasks_router())
        .merge(routes::clips_router())
        .merge(routes::media_router())
        .merge(routes::ai_edit_router())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let port: u16 = std::env::var("PORT")
        .ok().and_then(|p| p.parse().ok()).unwrap_or(8000);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    tracing::info!("NovaClip API listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
