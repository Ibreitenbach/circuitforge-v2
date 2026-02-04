use std::{path::{Path, PathBuf}, fs};
use anyhow::Context;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

#[derive(Debug, Clone)]
pub struct ArtifactRef {
  pub artifact_id: String,
  pub rel_path: String,
}

pub struct ArtifactStore {
  pool: SqlitePool,
  root: PathBuf,
}

impl ArtifactStore {
  pub fn new(pool: SqlitePool, root: impl AsRef<Path>) -> Self {
    Self { pool, root: root.as_ref().to_path_buf() }
  }

  /// Store bytes as a content-addressed artifact and link it to a run.
  /// Returns (artifact_id, rel_path).
  pub async fn put_bytes(
    &self,
    run_id: &str,
    kind: &str,
    mime: Option<&str>,
    label: Option<&str>,
    bytes: &[u8],
    meta: Option<serde_json::Value>,
  ) -> anyhow::Result<ArtifactRef> {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let artifact_id = hex::encode(digest);

    // store under <root>/<first2>/<artifact_id>
    let subdir = &artifact_id[0..2];
    let rel_path = format!("{}/{}", subdir, artifact_id);
    let abs_path = self.root.join(&rel_path);

    if let Some(parent) = abs_path.parent() {
      fs::create_dir_all(parent)?;
    }

    // Write only if missing (dedupe)
    if !abs_path.exists() {
      fs::write(&abs_path, bytes)
        .with_context(|| format!("writing artifact {}", abs_path.display()))?;
    }

    let now = now_ms_i64();
    let byte_len = bytes.len() as i64;
    let meta_json = meta.map(|v| v.to_string());

    // Insert metadata (idempotent)
    sqlx::query("INSERT OR IGNORE INTO artifacts(artifact_id, created_at_ms, byte_len, mime, kind, rel_path, meta_json) VALUES(?,?,?,?,?,?,?)")
      .bind(&artifact_id)
      .bind(now)
      .bind(byte_len)
      .bind(mime)
      .bind(kind)
      .bind(&rel_path)
      .bind(meta_json)
      .execute(&self.pool).await?;

    // Link to run
    sqlx::query("INSERT OR IGNORE INTO run_artifacts(run_id, artifact_id, label, created_at_ms) VALUES(?,?,?,?)")
      .bind(run_id)
      .bind(&artifact_id)
      .bind(label)
      .bind(now)
      .execute(&self.pool).await?;

    Ok(ArtifactRef { artifact_id, rel_path })
  }

  pub async fn put_text(
    &self,
    run_id: &str,
    kind: &str,
    mime: Option<&str>,
    label: Option<&str>,
    text: &str,
    meta: Option<serde_json::Value>,
  ) -> anyhow::Result<ArtifactRef> {
    self.put_bytes(run_id, kind, mime, label, text.as_bytes(), meta).await
  }

  pub fn abs_path(&self, rel_path: &str) -> PathBuf {
    self.root.join(rel_path)
  }

  /// Read artifact bytes by artifact_id
  pub async fn get_bytes(&self, artifact_id: &str) -> anyhow::Result<Vec<u8>> {
    // Look up rel_path from DB
    let row: Option<(String,)> = sqlx::query_as("SELECT rel_path FROM artifacts WHERE artifact_id = ?")
      .bind(artifact_id)
      .fetch_optional(&self.pool)
      .await?;

    let rel_path = row.context("artifact not found")?.0;
    let abs_path = self.root.join(&rel_path);
    let bytes = fs::read(&abs_path)
      .with_context(|| format!("reading artifact {}", abs_path.display()))?;

    Ok(bytes)
  }

  /// Read artifact as text
  pub async fn get_text(&self, artifact_id: &str) -> anyhow::Result<String> {
    let bytes = self.get_bytes(artifact_id).await?;
    Ok(String::from_utf8(bytes)?)
  }

  /// Read artifact as JSON
  pub async fn get_json<T: serde::de::DeserializeOwned>(&self, artifact_id: &str) -> anyhow::Result<T> {
    let text = self.get_text(artifact_id).await?;
    Ok(serde_json::from_str(&text)?)
  }
}

fn now_ms_i64() -> i64 {
  use std::time::{SystemTime, UNIX_EPOCH};
  SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64
}
