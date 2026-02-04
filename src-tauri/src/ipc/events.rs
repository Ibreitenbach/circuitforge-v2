use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventEnvelope<T: Clone> {
  pub run_id: String,
  pub seq: u64,
  pub ts: u64,
  pub r#type: String,
  pub payload: T,
}
