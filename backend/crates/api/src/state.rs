use novaclip_db::DbPool;
use uuid::Uuid;
use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    /// Sender half of the in-process job queue. API handlers push task IDs
    /// here; the worker loop reads from the receiver and processes them.
    pub job_tx: tokio::sync::mpsc::Sender<Uuid>,
    pub config: Config,
}
