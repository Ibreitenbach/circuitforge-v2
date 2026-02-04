use sqlx::{SqlitePool, Row};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use crate::schemas::construct::{ConstructSpec, ConstructKind, ConstructSpecRef};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Construct {
    pub construct_id: String,
    pub run_id: String,
    pub kind: ConstructKind,
    pub name: String,
    pub root: String,
    pub created_at_ms: i64,
}

pub struct Constructs {
    pool: SqlitePool,
}

impl Constructs {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        run_id: &str,
        kind: ConstructKind,
        name: &str,
        root: &str,
    ) -> anyhow::Result<String> {
        let construct_id = uuid::Uuid::new_v4().to_string();
        let now = now_ms();
        let kind_str = serde_json::to_value(&kind)?
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Failed to serialize ConstructKind"))?
            .to_string();

        sqlx::query(
            "INSERT INTO constructs(construct_id, run_id, kind, name, root, created_at_ms) VALUES(?,?,?,?,?,?)"
        )
            .bind(&construct_id)
            .bind(run_id)
            .bind(kind_str)
            .bind(name)
            .bind(root)
            .bind(now)
            .execute(&self.pool).await?;
        Ok(construct_id)
    }

    pub async fn get(&self, construct_id: &str) -> anyhow::Result<Option<Construct>> {
        let row = sqlx::query(
            "SELECT construct_id, run_id, kind, name, root, created_at_ms FROM constructs WHERE construct_id = ?"
        )
            .bind(construct_id)
            .fetch_optional(&self.pool).await?;

        if let Some(r) = row {
            let kind_str: String = r.get(2);
            let kind: ConstructKind = serde_json::from_value(serde_json::Value::String(kind_str))?;
            Ok(Some(Construct {
                construct_id: r.get(0),
                run_id: r.get(1),
                kind,
                name: r.get(3),
                root: r.get(4),
                created_at_ms: r.get(5),
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn list_for_run(&self, run_id: &str) -> anyhow::Result<Vec<Construct>> {
        let rows = sqlx::query(
            "SELECT construct_id, run_id, kind, name, root, created_at_ms FROM constructs WHERE run_id = ? ORDER BY created_at_ms ASC"
        )
            .bind(run_id)
            .fetch_all(&self.pool).await?;

        let mut out = vec![];
        for r in rows {
            let kind_str: String = r.get(2);
            let kind: ConstructKind = serde_json::from_value(serde_json::Value::String(kind_str))?;
            out.push(Construct {
                construct_id: r.get(0),
                run_id: r.get(1),
                kind,
                name: r.get(3),
                root: r.get(4),
                created_at_ms: r.get(5),
            });
        }
        Ok(out)
    }

    pub async fn add_spec(&self, construct_id: &str, spec: &ConstructSpec) -> anyhow::Result<(i64, String)> {
        let now = now_ms();
        let spec_json = serde_json::to_string(spec)?;
        let spec_hash = compute_hash(&spec_json);
        
        let next_rev = self.get_next_revision(construct_id).await?;
        
        sqlx::query(
            "INSERT INTO construct_specs(construct_id, revision, spec_json, spec_hash, created_at_ms) VALUES(?,?,?,?,?)"
        )
            .bind(construct_id)
            .bind(next_rev)
            .bind(spec_json)
            .bind(&spec_hash)
            .bind(now)
            .execute(&self.pool).await?;
        
        Ok((next_rev, spec_hash))
    }

    pub async fn get_latest_spec(&self, construct_id: &str) -> anyhow::Result<Option<ConstructSpecRef>> {
        let row = sqlx::query(
            "SELECT spec_json, revision, spec_hash, created_at_ms FROM construct_specs WHERE construct_id = ? ORDER BY revision DESC LIMIT 1"
        )
            .bind(construct_id)
            .fetch_optional(&self.pool).await?;

        if let Some(r) = row {
            let spec_json: String = r.get(0);
            let spec: ConstructSpec = serde_json::from_str(&spec_json)?;
            Ok(Some(ConstructSpecRef {
                spec,
                revision: r.get(1),
                spec_hash: r.get(2),
                created_at_ms: r.get(3),
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get_spec_at_revision(&self, construct_id: &str, revision: i64) -> anyhow::Result<Option<ConstructSpecRef>> {
        let row = sqlx::query(
            "SELECT spec_json, revision, spec_hash, created_at_ms FROM construct_specs WHERE construct_id = ? AND revision = ?"
        )
            .bind(construct_id)
            .bind(revision)
            .fetch_optional(&self.pool).await?;

        if let Some(r) = row {
            let spec_json: String = r.get(0);
            let spec: ConstructSpec = serde_json::from_str(&spec_json)?;
            Ok(Some(ConstructSpecRef {
                spec,
                revision: r.get(1),
                spec_hash: r.get(2),
                created_at_ms: r.get(3),
            }))
        } else {
            Ok(None)
        }
    }

    async fn get_next_revision(&self, construct_id: &str) -> anyhow::Result<i64> {
        let row = sqlx::query("SELECT COALESCE(MAX(revision), -1) + 1 FROM construct_specs WHERE construct_id = ?")
            .bind(construct_id)
            .fetch_one(&self.pool).await?;
        Ok(row.get(0))
    }
}

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64
}

fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}