use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConstructKind { Cli, Service, Library, Schema, Pipeline }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExportType { Command, HttpEndpoint, LibraryApi, FileArtifact }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InterfaceExport {
    pub id: String,
    pub r#type: ExportType,
    pub signature: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Interface {
    pub exports: Vec<InterfaceExport>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InvariantSeverity { Must, Should }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Invariant {
    pub id: String,
    pub statement: String,
    pub severity: InvariantSeverity,
    pub probe_ref: String, // e.g., "probe:<probe_id>"
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PassPolicyMode { All }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PassPolicy {
    pub mode: PassPolicyMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Acceptance {
    pub required_probes: Vec<String>,
    pub pass_policy: PassPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConstructDependency {
    pub construct_id: String,
    pub version: String, // pinned | latest_in_run
    pub mount: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CapsuleDependency {
    pub capsule_id: String,
    pub role: String, // generator | patch | probe
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Dependencies {
    pub constructs: Option<Vec<ConstructDependency>>,
    pub capsules: Option<Vec<CapsuleDependency>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConstructSpec {
    pub id: String,
    pub kind: ConstructKind,
    pub name: String,
    pub description: String,
    pub root: String,
    pub interface: Interface,
    pub invariants: Vec<Invariant>,
    pub acceptance: Acceptance,
    pub dependencies: Option<Dependencies>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConstructSpecWrapper {
    pub construct: ConstructSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConstructSpecRef {
    pub spec: ConstructSpec,
    pub revision: i64,
    pub spec_hash: String,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ConstructStatus { Draft, Active, Certified, Regressed, Archived }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateResult {
    pub status: ConstructStatus,
    pub missing_probes: Vec<String>,
    pub failing_probes: Vec<String>,
    pub satisfied_invariants: Vec<String>,
}
