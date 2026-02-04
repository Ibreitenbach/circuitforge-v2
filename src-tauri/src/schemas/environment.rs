use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NodeKind { Chip, Probe, Fuse, Relay, Bus, Battery, Terminal }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EdgeKind { Trace, BusLink }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SparkKind { Ac, Dc }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GateStatus { Locked, Open, Tripped }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProbeStatus { Idle, Running, Pass, Fail }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Direction { In, Out }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Vec2 { pub x: f32, pub y: f32 }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Size2 { pub w: f32, pub h: f32 }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardPort {
  pub id: String,
  pub name: String,
  pub dir: Direction,
  #[serde(rename = "type")]
  pub r#type: String,
  pub offset: Vec2,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeStatus {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub gate: Option<GateStatus>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub probe: Option<ProbeStatus>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub requires: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub satisfied: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardNode {
  pub id: String,
  pub kind: NodeKind,
  pub label: String,
  pub pos: Vec2,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub size: Option<Size2>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub module_id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub validator_id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub gate_id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub status: Option<NodeStatus>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub ports: Option<Vec<BoardPort>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EndpointRef {
  pub node_id: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub port_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardEdge {
  pub id: String,
  pub kind: EdgeKind,
  pub from: EndpointRef,
  pub to: EndpointRef,
  pub directed: bool,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub inline_gate_ids: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub points: Option<Vec<Vec2>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SparkState {
  pub id: String,
  pub kind: SparkKind,
  pub hp: i32,
  pub carrying: Vec<String>,
  pub at_node_id: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub on_edge_id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub t: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardSnapshot {
  pub run_id: String,
  pub nodes: Vec<BoardNode>,
  pub edges: Vec<BoardEdge>,
  pub sparks: Vec<SparkState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardDiff {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub upsert_nodes: Option<Vec<BoardNode>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub delete_node_ids: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub upsert_edges: Option<Vec<BoardEdge>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub delete_edge_ids: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub upsert_sparks: Option<Vec<SparkState>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisibleSlice { pub nodes: Vec<BoardNode>, pub edges: Vec<BoardEdge> }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignalMsg {
  pub from: String,
  pub payload: serde_json::Value,
  pub ts: i64,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub to_spark_id: Option<String>,
}

// ---- EnvironmentSpec (minimal; match doc, expand later) ----

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentSpec {
  pub id: String,
  pub name: String,
  pub language_profile: String,
  pub team: TeamConfig,
  pub board: BoardSpec,
  pub gates: Vec<GateSpec>,
  pub validators: Vec<ValidatorSpec>,
  pub permissions: PermissionsSpec,
  pub hp: HpSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamConfig {
  pub sparks: Vec<TeamSpark>,
  pub dc_config_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamSpark {
  pub id: String,
  pub kind: SparkKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardSpec { pub nodes: Vec<BoardNode>, pub edges: Vec<BoardEdge> }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GateSpec {
  pub id: String,
  pub kind: String, // fuse|relay
  pub predicate: Predicate,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub requires: Option<Vec<Predicate>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Predicate {
  ProbePass { probe_id: String },
  All { of: Vec<Predicate> },
  LintPass,
  TypecheckPass,
  CoverageAtLeast { pct: u32 },
  PatchCapsulePresent,
  EvidenceSetPresent { validators: Vec<String> },
  ChecklistPresent { rubric_at_least: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidatorSpec {
  pub id: String,
  pub kind: String, // test|composite
  #[serde(skip_serializing_if = "Option::is_none")]
  pub cmd: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub steps: Option<Vec<ValidatorStep>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidatorStep { pub cmd: Vec<String> }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionsSpec { pub ac: RolePerms, pub dc: RolePerms }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RolePerms {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub allow_edit_paths: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub deny_edit_paths: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub allow_config_paths: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub deny_commands: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HpSpec {
  pub max: i32,
  pub start: i32,
  pub damage: HpDamage,
  pub heal: HpHeal,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub anti_thrash: Option<AntiThrashSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HpDamage {
  pub unit_fail: i32,
  pub integration_fail: i32,
  pub quality_fail: i32,
  pub policy_violation_attempt: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HpHeal {
  pub unlock_gate_unit: i32,
  pub unlock_gate_integration: i32,
  pub clean_run_all: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AntiThrashSpec { pub repeat_failure_signature_after: u32, pub extra_damage: i32 }
