use sqlx::SqlitePool;
use serde_json::Value;

pub struct ToolRuns {
  pool: SqlitePool,
}

impl ToolRuns {
  pub fn new(pool: SqlitePool) -> Self { Self { pool } }

  pub async fn create(&self, run_id: &str, probe_id: Option<&str>, validator_id: Option<&str>, cmd: &Value) -> anyhow::Result<String> {
    let tool_run_id = uuid::Uuid::new_v4().to_string();
    let now = now_ms();
    sqlx::query("INSERT INTO tool_runs(tool_run_id, run_id, probe_id, validator_id, started_at_ms, status, cmd_json) VALUES(?,?,?,?,?,?,?)")
      .bind(&tool_run_id)
      .bind(run_id)
      .bind(probe_id)
      .bind(validator_id)
      .bind(now)
      .bind("running")
      .bind(cmd.to_string())
      .execute(&self.pool).await?;
    Ok(tool_run_id)
  }

  pub async fn finish(&self, tool_run_id: &str, status: &str, exit_code: Option<i64>, failure_signature: Option<&str>) -> anyhow::Result<()> {
    let now = now_ms();
    sqlx::query("UPDATE tool_runs SET finished_at_ms=?, status=?, exit_code=?, failure_signature=? WHERE tool_run_id=?")
      .bind(now)
      .bind(status)
      .bind(exit_code)
      .bind(failure_signature)
      .bind(tool_run_id)
      .execute(&self.pool).await?;
    Ok(())
  }

  pub async fn link_artifact(&self, tool_run_id: &str, artifact_id: &str, role: &str) -> anyhow::Result<()> {
    sqlx::query("INSERT OR IGNORE INTO tool_run_artifacts(tool_run_id, artifact_id, role) VALUES(?,?,?)")
      .bind(tool_run_id)
      .bind(artifact_id)
      .bind(role)
      .execute(&self.pool).await?;
    Ok(())
  }
}

fn now_ms() -> i64 {
  use std::time::{SystemTime, UNIX_EPOCH};
  SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64
}
