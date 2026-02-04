use sqlx::{SqlitePool, Row};
use serde_json::Value;
use anyhow::Context;

pub struct EventLog {
  pool: SqlitePool,
}

impl EventLog {
  pub fn new(pool: SqlitePool) -> Self { Self { pool } }

  pub async fn create_run_row(&self, run_id: &str, env_spec_path: &str, workspace_path: &str, seed: Option<u64>) -> anyhow::Result<()> {
    let now = now_ms();
    let mut tx = self.pool.begin().await?;
    sqlx::query("INSERT INTO runs(run_id, created_at_ms, env_spec_path, workspace_path, seed) VALUES(?,?,?,?,?)")
      .bind(run_id).bind(now as i64).bind(env_spec_path).bind(workspace_path).bind(seed.map(|s| s as i64))
      .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO run_seq(run_id, next_seq) VALUES(?,?)")
      .bind(run_id).bind(0_i64)
      .execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
  }

  /// Append one event atomically; returns assigned seq.
  pub async fn append_event(&self, run_id: &str, event_type: &str, payload: Value) -> anyhow::Result<u64> {
    let ts = now_ms();
    let payload_json = serde_json::to_string(&payload)?;
    let mut tx = self.pool.begin().await?;

    // Acquire write lock for the run via the transaction; SQLite serializes writes.
    let row = sqlx::query("SELECT next_seq FROM run_seq WHERE run_id = ?")
      .bind(run_id)
      .fetch_one(&mut *tx).await
      .with_context(|| format!("run_seq missing for run_id={run_id}"))?;
    let next_seq: i64 = row.get(0);

    sqlx::query("INSERT INTO events(run_id, seq, ts_ms, type, payload_json) VALUES(?,?,?,?,?)")
      .bind(run_id).bind(next_seq).bind(ts as i64).bind(event_type).bind(payload_json)
      .execute(&mut *tx).await?;

    sqlx::query("UPDATE run_seq SET next_seq = ? WHERE run_id = ?")
      .bind(next_seq + 1).bind(run_id)
      .execute(&mut *tx).await?;

    tx.commit().await?;
    Ok(next_seq as u64)
  }

  /// Append many events atomically; returns assigned seqs in order.
  pub async fn append_events(&self, run_id: &str, events: Vec<(&str, Value)>) -> anyhow::Result<Vec<u64>> {
    let ts = now_ms();
    let mut tx = self.pool.begin().await?;

    let row = sqlx::query("SELECT next_seq FROM run_seq WHERE run_id = ?")
      .bind(run_id)
      .fetch_one(&mut *tx).await
      .with_context(|| format!("run_seq missing for run_id={run_id}"))?;
    let mut next_seq: i64 = row.get(0);

    let mut seqs = Vec::with_capacity(events.len());
    for (ty, payload) in events {
      let payload_json = serde_json::to_string(&payload)?;
      sqlx::query("INSERT INTO events(run_id, seq, ts_ms, type, payload_json) VALUES(?,?,?,?,?)")
        .bind(run_id).bind(next_seq).bind(ts as i64).bind(ty).bind(payload_json)
        .execute(&mut *tx).await?;
      seqs.push(next_seq as u64);
      next_seq += 1;
    }

    sqlx::query("UPDATE run_seq SET next_seq = ? WHERE run_id = ?")
      .bind(next_seq).bind(run_id)
      .execute(&mut *tx).await?;

    tx.commit().await?;
    Ok(seqs)
  }

  pub async fn read_events(&self, run_id: &str, from_seq: u64, limit: u64) -> anyhow::Result<Vec<(u64, String, Value)>> {
    let rows = sqlx::query("SELECT seq, type, payload_json FROM events WHERE run_id = ? AND seq >= ? ORDER BY seq ASC LIMIT ?")
      .bind(run_id).bind(from_seq as i64).bind(limit as i64)
      .fetch_all(&self.pool).await?;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
      let seq: i64 = r.get(0);
      let ty: String = r.get(1);
      let payload_json: String = r.get(2);
      let payload: Value = serde_json::from_str(&payload_json)?;
      out.push((seq as u64, ty, payload));
    }
    Ok(out)
  }

  /// Get run info from runs table
  pub async fn get_run_info(&self, run_id: &str) -> anyhow::Result<RunInfo> {
    let row = sqlx::query("SELECT env_spec_path, workspace_path, seed FROM runs WHERE run_id = ?")
      .bind(run_id)
      .fetch_one(&self.pool).await
      .with_context(|| format!("run not found: {run_id}"))?;

    Ok(RunInfo {
      env_spec_path: row.get(0),
      workspace_path: row.get(1),
      seed: row.get::<Option<i64>, _>(2).map(|s| s as u64),
    })
  }

  /// Read all events for a run (for replay)
  pub async fn read_all_events(&self, run_id: &str) -> anyhow::Result<Vec<(u64, String, Value)>> {
    self.read_events(run_id, 0, u64::MAX).await
  }
}

#[derive(Debug, Clone)]
pub struct RunInfo {
  pub env_spec_path: String,
  pub workspace_path: String,
  pub seed: Option<u64>,
}

fn now_ms() -> u128 {
  use std::time::{SystemTime, UNIX_EPOCH};
  SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
}
