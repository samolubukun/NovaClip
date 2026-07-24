use anyhow::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;

pub type DbPool = SqlitePool;

pub async fn create_pool(database_url: &str) -> Result<DbPool> {
    let opts = SqliteConnectOptions::from_str(database_url)?
        .journal_mode(SqliteJournalMode::Wal)
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(16)
        .connect_with(opts)
        .await?;
    Ok(pool)
}

