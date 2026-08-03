use anyhow::Result;
use novaclip_db::pool::create_pool;
use tracing::info;
use uuid::Uuid;

/// Standalone worker binary — polls SQLite for queued tasks.
/// In the merged API+Worker setup (the normal case), you don't need to run this;
/// the API binary spawns the worker in-process. Use this only for separate deployments.
#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "novaclip_worker=info".into()),
        )
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://novaclip.db".into());

    info!("Connecting to SQLite: {}", database_url);
    let db = create_pool(&database_url).await?;

    // Run migrations on startup
    sqlx::migrate!("../../migrations").run(&db).await?;

    info!("NovaClip standalone worker started — polling DB for queued tasks...");

    let (tx, rx) = tokio::sync::mpsc::channel::<Uuid>(256);

    // Spawn the worker processing loop from the shared lib
    let db_worker = db.clone();
    tokio::spawn(async move {
        novaclip_worker::run_worker(db_worker, rx).await;
    });

    // Poll the DB for queued tasks
    loop {
        let id_str: Option<String> = sqlx::query_scalar(
            "SELECT id FROM tasks WHERE status = 'queued' ORDER BY created_at ASC LIMIT 1",
        )
        .fetch_optional(&db)
        .await
        .ok()
        .flatten();

        if let Some(id_str) = id_str {
            // Optimistically claim the task
            let claimed = sqlx::query(
                "UPDATE tasks SET status = 'processing', updated_at = datetime('now') \
                 WHERE id = ? AND status = 'queued'",
            )
            .bind(&id_str)
            .execute(&db)
            .await;

            if let Ok(result) = claimed {
                if result.rows_affected() > 0 {
                    if let Ok(task_id) = Uuid::parse_str(&id_str) {
                        info!("Claimed task {} from DB queue", task_id);
                        tx.send(task_id).await.ok();
                    }
                }
            }
        } else {
            // Nothing queued — sleep briefly before polling again
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }
}
