pub mod event_log;
pub mod artifacts;
pub mod tool_runs;
pub mod constructs;

use std::{path::Path, time::Duration};
use sqlx::{sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous}, SqlitePool};

use crate::storage::{
    event_log::EventLog, 
    artifacts::ArtifactStore, 
    tool_runs::ToolRuns,
    constructs::Constructs,
};

pub struct Storage {
  pub pool: SqlitePool,
  pub event_log: EventLog,
  pub artifacts: ArtifactStore,
  pub tool_runs: ToolRuns,
  pub constructs: Constructs,
}

impl Storage {
  pub async fn new(db_path: &str, artifact_root: &str) -> anyhow::Result<Self> {
    if let Some(parent) = Path::new(db_path).parent() { std::fs::create_dir_all(parent)?; }
    std::fs::create_dir_all(artifact_root)?;

    // Use pragma() method to ensure foreign_keys is set on EVERY connection in the pool
    // This is applied during connection setup, not just on one connection
    let opts = SqliteConnectOptions::new()
      .filename(db_path)
      .create_if_missing(true)
      .journal_mode(SqliteJournalMode::Wal)
      .synchronous(SqliteSynchronous::Normal)
      .busy_timeout(Duration::from_millis(250))
      .pragma("foreign_keys", "ON");  // Applied to every connection

    let pool = SqlitePoolOptions::new()
      .max_connections(4)
      .acquire_timeout(Duration::from_secs(5))
      .connect_with(opts).await?;

    // NOTE: path is relative to src-tauri/; adjust if you place migrations elsewhere.
    sqlx::migrate!("./src/storage/migrations").run(&pool).await?;

    let event_log = EventLog::new(pool.clone());
    let artifacts = ArtifactStore::new(pool.clone(), artifact_root);
    let tool_runs = ToolRuns::new(pool.clone());
    let constructs = Constructs::new(pool.clone());

    Ok(Self { pool, event_log, artifacts, tool_runs, constructs })
  }
}
