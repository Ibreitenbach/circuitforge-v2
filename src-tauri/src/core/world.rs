use std::{collections::{HashMap, VecDeque, HashSet}, fs, path::{Path, PathBuf}};

use anyhow::Context;
use globset::{Glob, GlobSetBuilder};
use serde_json::json;
use sha2::Digest;
use thiserror::Error;
use tokio::sync::RwLock;

use crate::schemas::environment::*;
use crate::storage::Storage;
use crate::sandbox::runner::{SandboxRunner, ToolRun};

#[derive(Error, Debug)]
pub enum WorldError {
    #[error("ACL Violation: {0}")]
    AclViolation(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    #[error("Security: Path Traversal Attempt: {0}")]
    PathTraversal(String),
}

/// Thread-safe WorldService using internal mutability to allow concurrent access
/// even during long-running operations like sandbox probes.
pub struct WorldService {
  runs: RwLock<HashMap<String, RunState>>,
  runs_root: PathBuf,
  next_seq: RwLock<u64>,
}

#[derive(Clone)]
struct RunState {
  env_spec_path: String,
  workspace_path: String,
  env: EnvironmentSpec,
  snapshot: BoardSnapshot,
  last_edits: Vec<String>,
  failure_history: Vec<String>,
  inbox: Vec<SignalMsg>,
  seed: Option<u64>,
}

pub struct PatchOutcome { pub patch_id: String, pub item_id: String, pub touched_paths: Vec<String> }

pub struct ProbeOutcome {
  pub status: String,
  pub evidence_item_ids: Vec<String>,
  pub artifact_refs: Vec<String>,
  pub failure_signature: Option<String>,
  pub diff: BoardDiff,
  pub seq: u64,
  pub event_payload: serde_json::Value,
}

pub struct DiffOutcome { pub diff: BoardDiff, pub seq: u64, pub run_id: String }

impl WorldService {
  pub fn new(runs_root: &PathBuf) -> Self {
    Self {
      runs: RwLock::new(HashMap::new()),
      runs_root: runs_root.clone(),
      next_seq: RwLock::new(1),
    }
  }

  pub async fn init_run(&self, run_id: &str, env_spec_path: &str, seed: Option<u64>) -> anyhow::Result<(String, BoardSnapshot, u64)> {
    let env_text = fs::read_to_string(env_spec_path).with_context(|| format!("read env spec {env_spec_path}"))?;
    let env: EnvironmentSpec = serde_yaml::from_str(&env_text).context("parse env spec yaml")?;

    let workspace_path = self.runs_root.join(run_id).join("workspace");
    
    let env_dir = Path::new(env_spec_path).parent()
      .context("Invalid env_spec_path: no parent directory")?;
      
    fs::create_dir_all(&workspace_path)?;
    
    let repo_template = env_dir.join("repo_template");
    if repo_template.exists() {
      copy_dir(&repo_template, &workspace_path)?;
    }

    let mut snapshot = BoardSnapshot {
      run_id: run_id.to_string(),
      nodes: env.board.nodes.clone(),
      edges: env.board.edges.clone(),
      sparks: env.team.sparks.iter().map(|s| SparkState {
        id: s.id.clone(),
        kind: s.kind.clone(),
        hp: env.hp.start,
        carrying: vec![],
        at_node_id: env.board.nodes.first().map(|n| n.id.clone()).unwrap_or_else(|| "start".into()),
        on_edge_id: None,
        t: None,
      }).collect(),
    };

    for n in snapshot.nodes.iter_mut() {
      match n.kind {
        NodeKind::Probe => {
          let st = n.status.get_or_insert(NodeStatus{ gate: None, probe: None, requires: None, satisfied: None });
          st.probe = Some(ProbeStatus::Idle);
        }
        NodeKind::Fuse | NodeKind::Relay => {
          let st = n.status.get_or_insert(NodeStatus{ gate: None, probe: None, requires: None, satisfied: None });
          st.gate = Some(GateStatus::Locked);
        }
        _ => {}
      }
    }

    let state = RunState {
      env_spec_path: env_spec_path.to_string(),
      workspace_path: workspace_path.to_string_lossy().to_string(),
      env: env.clone(),
      snapshot: snapshot.clone(),
      last_edits: vec![],
      failure_history: vec![],
      inbox: vec![],
      seed,
    };
    let workspace_path_str = state.workspace_path.clone();
    self.runs.write().await.insert(run_id.to_string(), state);

    Ok((workspace_path_str, snapshot, seed.unwrap_or(0)))
  }

  pub async fn load_run(&self, storage: &Storage, run_id: &str) -> anyhow::Result<BoardSnapshot> {
    {
        let runs = self.runs.read().await;
        if let Some(st) = runs.get(run_id) {
          return Ok(st.snapshot.clone());
        }
    }

    let run_info = storage.event_log.get_run_info(run_id).await?;
    let env_text = fs::read_to_string(&run_info.env_spec_path)
      .with_context(|| format!("read env spec {}", run_info.env_spec_path))?;
    let env: EnvironmentSpec = serde_yaml::from_str(&env_text).context("parse env spec yaml")?;

    let mut snapshot = BoardSnapshot {
      run_id: run_id.to_string(),
      nodes: env.board.nodes.clone(),
      edges: env.board.edges.clone(),
      sparks: env.team.sparks.iter().map(|s| SparkState {
        id: s.id.clone(),
        kind: s.kind.clone(),
        hp: env.hp.start,
        carrying: vec![],
        at_node_id: env.board.nodes.first().map(|n| n.id.clone()).unwrap_or_else(|| "start".into()),
        on_edge_id: None,
        t: None,
      }).collect(),
    };

    for n in snapshot.nodes.iter_mut() {
      match n.kind {
        NodeKind::Probe => {
          let st = n.status.get_or_insert(NodeStatus{ gate: None, probe: None, requires: None, satisfied: None });
          st.probe = Some(ProbeStatus::Idle);
        }
        NodeKind::Fuse | NodeKind::Relay => {
          let st = n.status.get_or_insert(NodeStatus{ gate: None, probe: None, requires: None, satisfied: None });
          st.gate = Some(GateStatus::Locked);
        }
        _ => {}
      }
    }

    let mut failure_history = vec![];
    let events = storage.event_log.read_all_events(run_id).await?;
    let mut max_seq = 0;
    for (seq, event_type, payload) in events {
      if seq > max_seq { max_seq = seq; }
      self.apply_event(&mut snapshot, &mut failure_history, &event_type, &payload);
    }
    *self.next_seq.write().await = max_seq + 1;

    let state = RunState {
      env_spec_path: run_info.env_spec_path,
      workspace_path: run_info.workspace_path,
      env,
      snapshot: snapshot.clone(),
      last_edits: vec![],
      failure_history,
      inbox: vec![],
      seed: run_info.seed,
    };
    self.runs.write().await.insert(run_id.to_string(), state);

    Ok(snapshot)
  }

  pub async fn clear_run(&self, run_id: &str) {
    self.runs.write().await.remove(run_id);
  }

  fn apply_event(&self, snapshot: &mut BoardSnapshot, failure_history: &mut Vec<String>, event_type: &str, payload: &serde_json::Value) {
    match event_type {
      "ProbeFinished" => {
        if let (Some(probe_id), Some(status)) = (
          payload.get("probeId").and_then(|p| p.as_str()),
          payload.get("status").and_then(|s| s.as_str()),
        ) {
          if status == "fail" {
            if let Some(sig) = payload.get("failureSignature").and_then(|s| s.as_str()) {
              failure_history.push(sig.to_string());
            }
          }

          // Apply HP delta to the spark that ran the probe
          if let (Some(spark_id), Some(hp_delta)) = (
            payload.get("sparkId").and_then(|s| s.as_str()),
            payload.get("hpDelta").and_then(|h| h.as_i64()),
          ) {
            if let Some(spark) = snapshot.sparks.iter_mut().find(|s| s.id == spark_id) {
              spark.hp = (spark.hp + hp_delta as i32).max(0);
            }
          }

          for n in snapshot.nodes.iter_mut() {
            if n.id == probe_id {
              let st = n.status.get_or_insert(NodeStatus{ gate: None, probe: None, requires: None, satisfied: None });
              st.probe = Some(if status == "pass" { ProbeStatus::Pass } else { ProbeStatus::Fail });
            }
          }
          if let Some(opened_gates) = payload.get("openedGateIds").and_then(|g| g.as_array()) {
            for gate_id in opened_gates.iter().filter_map(|g| g.as_str()) {
              for n in snapshot.nodes.iter_mut() {
                if let Some(gid) = &n.gate_id {
                  if gid == gate_id {
                    let st = n.status.get_or_insert(NodeStatus{ gate: None, probe: None, requires: None, satisfied: None });
                    st.gate = Some(GateStatus::Open);
                  }
                }
              }
            }
          }
        }
      }
      "RelayClosed" => {
        if let Some(relay_id) = payload.get("relayId").and_then(|r| r.as_str()) {
          for n in snapshot.nodes.iter_mut() {
            if n.id == relay_id {
              let st = n.status.get_or_insert(NodeStatus{ gate: None, probe: None, requires: None, satisfied: None });
              st.gate = Some(GateStatus::Open);
            }
          }
        }
      }
      "PatchCreated" => {
        if let (Some(spark_id), Some(item_id)) = (
          payload.get("sparkId").and_then(|s| s.as_str()),
          payload.get("itemId").and_then(|i| i.as_str()),
        ) {
          if let Some(spark) = snapshot.sparks.iter_mut().find(|s| s.id == spark_id) {
            if !spark.carrying.contains(&item_id.to_string()) {
              spark.carrying.push(item_id.to_string());
            }
          }
        }
      }
      "SparkMoved" => {
        if let (Some(spark_id), Some(to_node_id)) = (
          payload.get("sparkId").and_then(|s| s.as_str()),
          payload.get("toNodeId").and_then(|n| n.as_str()),
        ) {
          if let Some(spark) = snapshot.sparks.iter_mut().find(|s| s.id == spark_id) {
            spark.at_node_id = to_node_id.to_string();
          }
        }
      }
      "ObjectEnergized" => {
          if let Some(object_id) = payload.get("objectId").and_then(|o| o.as_str()) {
              for n in snapshot.nodes.iter_mut() {
                  if n.id == object_id {
                      let mut status = n.status.clone().unwrap_or_default();
                      status.satisfied = Some(true);
                      n.status = Some(status);
                  }
              }
          }
      }
      "PolicyViolation" => {
          // Replay HP damage from policy violations
          if let (Some(spark_id), Some(hp_delta)) = (
            payload.get("sparkId").and_then(|s| s.as_str()),
            payload.get("hpDelta").and_then(|h| h.as_i64()),
          ) {
            if let Some(spark) = snapshot.sparks.iter_mut().find(|s| s.id == spark_id) {
              spark.hp = (spark.hp + hp_delta as i32).max(0);
            }
          }
      }
      _ => {}
    }
  }

  pub async fn observe(&self, run_id: &str, spark_id: &str) -> anyhow::Result<(SparkState, VisibleSlice, Vec<SignalMsg>)> {
    let runs = self.runs.read().await;
    let st = runs.get(run_id).context("unknown run")?;
    let spark = st.snapshot.sparks.iter().find(|s| s.id == spark_id).context("unknown spark")?.clone();
    
    let visible_node_ids = self.find_visible_node_ids(&st.snapshot, &spark.at_node_id, 2);
    
    let visible_nodes: Vec<BoardNode> = st.snapshot.nodes.iter()
        .filter(|n| visible_node_ids.contains(&n.id))
        .cloned()
        .collect();
        
    let visible_edges: Vec<BoardEdge> = st.snapshot.edges.iter()
        .filter(|e| visible_node_ids.contains(&e.from.node_id) || visible_node_ids.contains(&e.to.node_id))
        .cloned()
        .collect();
        
    // Real signal filtering: only see signals relevant to the spark or broad-spectrum
    let visible_signals = st.inbox.iter()
        .filter(|m| m.to_spark_id.as_deref() == Some(spark_id) || m.to_spark_id.is_none())
        .cloned()
        .collect();
    
    Ok((spark, VisibleSlice { nodes: visible_nodes, edges: visible_edges }, visible_signals))
  }
  
  fn find_visible_node_ids(&self, snapshot: &BoardSnapshot, start_node_id: &str, max_dist: u32) -> HashSet<String> {
      let mut visible = HashSet::new();
      let mut queue = VecDeque::new();
      let mut dists = HashMap::new();
      
      queue.push_back(start_node_id.to_string());
      dists.insert(start_node_id.to_string(), 0);
      visible.insert(start_node_id.to_string());
      
      while let Some(curr) = queue.pop_front() {
          let d = *dists.get(&curr).unwrap();
          if d >= max_dist { continue; }
          
          for edge in &snapshot.edges {
              let neighbor = if edge.from.node_id == curr {
                  Some(&edge.to.node_id)
              } else if edge.to.node_id == curr && !edge.directed {
                  Some(&edge.from.node_id)
              } else {
                  None
              };
              
              if let Some(next_id) = neighbor {
                  if !visible.contains(next_id) {
                      visible.insert(next_id.clone());
                      dists.insert(next_id.clone(), d + 1);
                      queue.push_back(next_id.clone());
                  }
              }
          }
      }
      visible
  }

  pub async fn act(&self, storage: &Storage, action: crate::ipc::commands::WorldAction) -> Result<DiffOutcome, WorldError> {
    let mut seq_lock = self.next_seq.write().await;
    let seq = *seq_lock;
    *seq_lock += 1;
    
    match action {
      crate::ipc::commands::WorldAction::Move { run_id, spark_id, to_node_id } => {
        // Persist first - if this fails, don't apply in memory
        storage.event_log.append_event(&run_id, "SparkMoved", json!({
          "sparkId": spark_id,
          "toNodeId": to_node_id
        })).await.map_err(|e| WorldError::Internal(format!("DB persist failed: {}", e)))?;

        let mut runs = self.runs.write().await;
        let st = runs.get_mut(&run_id).ok_or_else(|| WorldError::NotFound(run_id.clone()))?;

        let mut sparks = st.snapshot.sparks.clone();
        if let Some(s) = sparks.iter_mut().find(|s| s.id == spark_id) {
          s.at_node_id = to_node_id;
        }
        st.snapshot.sparks = sparks.clone();
        Ok(DiffOutcome { diff: BoardDiff { upsert_nodes: None, delete_node_ids: None, upsert_edges: None, delete_edge_ids: None, upsert_sparks: Some(sparks) }, seq, run_id })
      }
      crate::ipc::commands::WorldAction::Energize { run_id, spark_id, object_id } => {
        // Persist first - if this fails, don't apply in memory
        storage.event_log.append_event(&run_id, "ObjectEnergized", json!({
          "sparkId": spark_id,
          "objectId": object_id
        })).await.map_err(|e| WorldError::Internal(format!("DB persist failed: {}", e)))?;

        let mut runs = self.runs.write().await;
        let st = runs.get_mut(&run_id).ok_or_else(|| WorldError::NotFound(run_id.clone()))?;

        let mut upsert_nodes = vec![];
        for n in st.snapshot.nodes.iter_mut() {
            if n.id == object_id {
                let mut status = n.status.clone().unwrap_or_default();
                status.satisfied = Some(true);
                n.status = Some(status);
                upsert_nodes.push(n.clone());
            }
        }

        Ok(DiffOutcome { diff: BoardDiff { upsert_nodes: Some(upsert_nodes), delete_node_ids: None, upsert_edges: None, delete_edge_ids: None, upsert_sparks: None }, seq, run_id })
      }
    }
  }

  pub async fn ac_edit_files(&self, run_id: &str, spark_id: &str, edits: Vec<crate::ipc::commands::FileEdit>) -> Result<(), WorldError> {
    let mut runs = self.runs.write().await;
    let st = runs.get_mut(run_id).ok_or_else(|| WorldError::NotFound(run_id.to_string()))?;
    self.assert_ac(st, spark_id)?;
    
    let allow = globset_from_opt_secure(st.env.permissions.ac.allow_edit_paths.as_ref())
        .map_err(|e| WorldError::Internal(e.to_string()))?;
    
    for e in edits {
      if !allow.is_match(&e.path) {
        return Err(WorldError::AclViolation(format!("AC cannot edit path {}", e.path)));
      }
      let abs = secure_join(&st.workspace_path, &e.path)?;
      if let Some(parent) = abs.parent() { fs::create_dir_all(parent).map_err(|e| WorldError::Internal(e.to_string()))?; }
      fs::write(abs, e.contents).map_err(|e| WorldError::Internal(e.to_string()))?;
      st.last_edits.push(e.path);
    }
    Ok(())
  }

  fn assert_ac(&self, st: &RunState, spark_id: &str) -> Result<(), WorldError> {
    let s = st.snapshot.sparks.iter().find(|s| s.id == spark_id).ok_or_else(|| WorldError::NotFound(spark_id.to_string()))?;
    if !matches!(s.kind, SparkKind::Ac) {
        return Err(WorldError::AclViolation("spark is not AC".into()));
    }
    Ok(())
  }

  fn assert_dc(&self, st: &RunState, spark_id: &str) -> Result<(), WorldError> {
    let s = st.snapshot.sparks.iter().find(|s| s.id == spark_id).ok_or_else(|| WorldError::NotFound(spark_id.to_string()))?;
    if !matches!(s.kind, SparkKind::Dc) {
        return Err(WorldError::AclViolation("spark is not DC".into()));
    }
    Ok(())
  }

  pub async fn ac_create_patch(&self, storage: &Storage, run_id: &str, spark_id: &str, message: &str) -> anyhow::Result<PatchOutcome> {
    let mut runs = self.runs.write().await;
    let st = runs.get_mut(run_id).context("unknown run")?;
    let touched = st.last_edits.clone();
    anyhow::ensure!(!touched.is_empty(), "no edits staged; nothing to patch");

    let mut files = vec![];
    for p in touched.iter() {
      let abs = secure_join(&st.workspace_path, p).context("Security: path traversal")?;
      let contents = fs::read_to_string(&abs).with_context(|| format!("read {}", abs.display()))?;
      files.push(json!({ "path": p, "contents": contents }));
    }
    let capsule = json!({ "message": message, "files": files, "touchedPaths": touched });
    let bytes = serde_json::to_vec_pretty(&capsule)?;

    let artifact_ref = storage.artifacts.put_bytes(
      run_id,
      "patch",
      Some("application/json"),
      Some(&format!("patch/{}", &hex::encode(sha2::Sha256::digest(&bytes))[..16])),
      &bytes,
      Some(json!({ "sparkId": spark_id, "message": message })),
    ).await?;

    let patch_id = artifact_ref.artifact_id.clone();
    let item_id = patch_id.clone();

    if let Some(s) = st.snapshot.sparks.iter_mut().find(|s| s.id == spark_id) {
      s.carrying.push(item_id.clone());
    }
    st.last_edits.clear();

    Ok(PatchOutcome { patch_id, item_id, touched_paths: touched })
  }

  pub async fn dc_apply_patch(&self, storage: &Storage, run_id: &str, spark_id: &str, patch_id: &str) -> anyhow::Result<Vec<String>> {
    let mut runs = self.runs.write().await;
    let st = runs.get_mut(run_id).context("unknown run")?;
    self.assert_dc(st, spark_id).map_err(|e| anyhow::anyhow!("{}", e))?;
    let dc = st.snapshot.sparks.iter().find(|s| s.id == spark_id).context("missing spark")?;
    anyhow::ensure!(dc.carrying.iter().any(|x| x == patch_id), "DC not carrying patch_id");

    let capsule: serde_json::Value = storage.artifacts.get_json(patch_id).await
      .context("failed to read patch artifact")?;

    let touched_paths: Vec<String> = capsule.get("touchedPaths")
        .and_then(|t| t.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();

    let files = capsule.get("files").and_then(|f| f.as_array()).context("patch has no files array")?;
    for file_entry in files {
      let path = file_entry.get("path").and_then(|p| p.as_str()).context("file entry missing path")?;
      let contents = file_entry.get("contents").and_then(|c| c.as_str()).context("file entry missing contents")?;

      let abs_path = secure_join(&st.workspace_path, path).context("Security: path traversal in patch")?;
      if let Some(parent) = abs_path.parent() {
        fs::create_dir_all(parent)?;
      }
      fs::write(&abs_path, contents)
        .with_context(|| format!("writing patched file {}", abs_path.display()))?;
    }

    Ok(touched_paths)
  }

  pub async fn dc_run_probe(&self, storage: &Storage, sandbox: &SandboxRunner, run_id: &str, spark_id: &str, probe_id: &str) -> anyhow::Result<ProbeOutcome> {
    let (cmds, validator_id, validator_kind, workspace_path) = {
        let runs = self.runs.read().await;
        let st = runs.get(run_id).context("unknown run")?;
        let probe_node = st.snapshot.nodes.iter().find(|n| n.id == probe_id).context("probe node not found")?;
        let vid = probe_node.validator_id.clone().context("probe missing validator_id")?;
        let v = st.env.validators.iter().find(|v| v.id == vid).context("validator not found")?;
        let c: Vec<Vec<String>> = if let Some(cmd) = &v.cmd { vec![cmd.clone()] }
          else if let Some(steps) = &v.steps { steps.iter().map(|s| s.cmd.clone()).collect() }
          else { anyhow::bail!("validator has no cmd/steps"); };
        (c, vid, v.kind.clone(), st.workspace_path.clone())
    };

    let tool_run_id = storage.tool_runs.create(run_id, Some(probe_id), Some(&validator_id), &serde_json::to_value(&cmds)?).await?;

    {
        let mut runs = self.runs.write().await;
        let st = runs.get_mut(run_id).context("unknown run")?;
        for n in st.snapshot.nodes.iter_mut() {
          if n.id == probe_id {
            let stt = n.status.get_or_insert(NodeStatus{ gate: None, probe: None, requires: None, satisfied: None });
            stt.probe = Some(ProbeStatus::Running);
          }
        }
    }

    let mut all_ok = true;
    let mut artifact_refs = vec![];
    let mut last_exit = 0;

    for (i, cmd) in cmds.iter().enumerate() {
      let result = sandbox.run(ToolRun{
        cmd: cmd.clone(),
        cwd: workspace_path.clone(),
        env: vec![],
        timeout_ms: 60_000,
        max_bytes_stdout: 256_000,
        max_bytes_stderr: 256_000,
      }).await?;

      last_exit = result.exit_code;
      if result.exit_code != 0 { 
          all_ok = false;
          break;
      }

      let stdout_ref = storage.artifacts.put_bytes(run_id, "stdout", Some("text/plain"), Some(&format!("{probe_id}/step{i}/stdout")), &result.stdout, None).await?;
      let stderr_ref = storage.artifacts.put_bytes(run_id, "stderr", Some("text/plain"), Some(&format!("{probe_id}/step{i}/stderr")), &result.stderr, None).await?;

      storage.tool_runs.link_artifact(&tool_run_id, &stdout_ref.artifact_id, "stdout").await?;
      storage.tool_runs.link_artifact(&tool_run_id, &stderr_ref.artifact_id, "stderr").await?;

      artifact_refs.push(stdout_ref.rel_path.clone());
      artifact_refs.push(stderr_ref.rel_path.clone());
    }

    // Phase 1: Compute all event data WITHOUT modifying memory
    let (status, failure_signature, hp_delta, opened_gate_ids, evidence_item_id) = {
        let runs = self.runs.read().await;
        let st = runs.get(run_id).context("unknown run")?;

        let status = if all_ok { "pass" } else { "fail" };
        let failure_signature = if all_ok { None } else { Some(format!("{}_exit_{}", validator_id, last_exit)) };

        let mut hp_delta: i32 = 0;
        if !all_ok {
          let base_damage = match validator_kind.as_str() {
            "unit" | "test" => st.env.hp.damage.unit_fail,
            "integration" => st.env.hp.damage.integration_fail,
            "quality" | "lint" | "typecheck" | "coverage" => st.env.hp.damage.quality_fail,
            _ => st.env.hp.damage.unit_fail,
          };

          let anti_thrash_damage = if let (Some(sig), Some(at_spec)) = (&failure_signature, &st.env.hp.anti_thrash) {
            let repeat_count = st.failure_history.iter().filter(|s| *s == sig).count() as u32;
            if repeat_count >= at_spec.repeat_failure_signature_after { at_spec.extra_damage } else { 0 }
          } else { 0 };

          hp_delta = -(base_damage + anti_thrash_damage) as i32;
        }

        // Compute opened gates (read-only from env.gates)
        let mut opened_gate_ids: Vec<(String, String)> = vec![];
        for g in st.env.gates.iter() {
          if matches!(&g.predicate, Predicate::ProbePass{probe_id: p} if p == probe_id) && all_ok {
            opened_gate_ids.push((g.id.clone(), g.kind.clone()));
          }
        }

        // Store evidence artifact
        let evidence = json!({
          "validatorId": validator_id,
          "probeId": probe_id,
          "status": status,
          "artifactRefs": artifact_refs,
          "toolRunId": tool_run_id,
          "failureSignature": failure_signature,
        });
        let evidence_bytes = serde_json::to_vec(&evidence)?;
        let evidence_ref = storage.artifacts.put_bytes(run_id, "evidence", Some("application/json"), Some(&format!("{probe_id}/evidence")), &evidence_bytes, None).await?;
        storage.tool_runs.link_artifact(&tool_run_id, &evidence_ref.artifact_id, "report").await?;

        (status.to_string(), failure_signature, hp_delta, opened_gate_ids, evidence_ref.artifact_id)
    };

    // Finish tool run
    storage.tool_runs.finish(&tool_run_id, &status, Some(last_exit as i64), failure_signature.as_deref()).await?;

    // Phase 2: Persist event to DB BEFORE memory updates
    let opened_gate_id_list: Vec<String> = opened_gate_ids.iter().map(|(id, _)| id.clone()).collect();
    let event_payload = json!({
      "probeId": probe_id,
      "validatorId": validator_id,
      "sparkId": spark_id,
      "status": status,
      "evidenceItemId": evidence_item_id,
      "artifactRefs": artifact_refs,
      "openedGateIds": opened_gate_id_list,
      "hpDelta": hp_delta,
    });

    let seq = storage.event_log.append_event(run_id, "ProbeFinished", event_payload.clone())
        .await.context("DB persist failed for ProbeFinished")?;

    // Phase 3: Now safe to update memory (DB is source of truth)
    let mut runs = self.runs.write().await;
    let st = runs.get_mut(run_id).context("unknown run")?;

    if let Some(sig) = &failure_signature {
      st.failure_history.push(sig.clone());
    }

    for s in st.snapshot.sparks.iter_mut() {
      if s.id == spark_id {
        s.hp = (s.hp + hp_delta).clamp(0, st.env.hp.max);
      }
    }

    if let Some(s) = st.snapshot.sparks.iter_mut().find(|s| s.id == spark_id) {
      s.carrying.push(evidence_item_id.clone());
    }

    let mut upsert_nodes = vec![];
    for n in st.snapshot.nodes.iter_mut() {
      if n.id == probe_id {
        let stt = n.status.get_or_insert(NodeStatus{ gate: None, probe: None, requires: None, satisfied: None });
        stt.probe = Some(if status == "pass" { ProbeStatus::Pass } else { ProbeStatus::Fail });
        upsert_nodes.push(n.clone());
      }
    }

    for (_, gate_kind) in &opened_gate_ids {
      let heal = match gate_kind.as_str() {
        "unit" | "fuse" => st.env.hp.heal.unlock_gate_unit,
        "integration" => st.env.hp.heal.unlock_gate_integration,
        _ => st.env.hp.heal.unlock_gate_unit,
      };
      for s in st.snapshot.sparks.iter_mut() {
        if s.id == spark_id {
          s.hp = (s.hp + heal).clamp(0, st.env.hp.max);
        }
      }
    }

    for n in st.snapshot.nodes.iter_mut() {
      if let Some(gid) = &n.gate_id {
        if opened_gate_id_list_contains(&opened_gate_ids, gid) {
          let stt = n.status.get_or_insert(NodeStatus{ gate: None, probe: None, requires: None, satisfied: None });
          stt.gate = Some(GateStatus::Open);
          upsert_nodes.push(n.clone());
        }
      }
    }

    let diff = BoardDiff {
      upsert_nodes: Some(dedup_nodes(upsert_nodes)),
      delete_node_ids: None,
      upsert_edges: Some(st.snapshot.edges.clone()),
      delete_edge_ids: None,
      upsert_sparks: Some(st.snapshot.sparks.clone()),
    };

    Ok(ProbeOutcome {
      status,
      evidence_item_ids: vec![evidence_item_id],
      artifact_refs,
      failure_signature,
      diff,
      seq,
      event_payload,
    })
  }

  pub async fn dc_close_relay(&self, storage: &Storage, run_id: &str, spark_id: &str, relay_id: &str) -> anyhow::Result<DiffOutcome> {
    // Verify DC role before any changes
    {
        let runs = self.runs.read().await;
        let st = runs.get(run_id).context("unknown run")?;
        self.assert_dc(st, spark_id).map_err(|e| anyhow::anyhow!("{}", e))?;
    }

    // Persist first - if this fails, don't apply in memory
    let seq = storage.event_log.append_event(run_id, "RelayClosed", json!({
      "sparkId": spark_id,
      "relayId": relay_id
    })).await.context("DB persist failed for RelayClosed")?;

    // Now safe to update memory
    let mut runs = self.runs.write().await;
    let st = runs.get_mut(run_id).context("unknown run")?;

    let mut upsert_nodes = vec![];
    for n in st.snapshot.nodes.iter_mut() {
      if n.id == relay_id {
        let stt = n.status.get_or_insert(NodeStatus{ gate: None, probe: None, requires: None, satisfied: None });
        stt.gate = Some(GateStatus::Open);
        upsert_nodes.push(n.clone());
      }
    }

    let diff = BoardDiff { upsert_nodes: Some(upsert_nodes), delete_node_ids: None, upsert_edges: None, delete_edge_ids: None, upsert_sparks: Some(st.snapshot.sparks.clone()) };
    Ok(DiffOutcome { diff, seq, run_id: run_id.to_string() })
  }

  pub async fn dc_edit_config(&self, run_id: &str, spark_id: &str, reason: &str, edits: Vec<crate::ipc::commands::FileEdit>) -> Result<Vec<String>, WorldError> {
    let mut runs = self.runs.write().await;
    let st = runs.get_mut(run_id).ok_or_else(|| WorldError::NotFound(run_id.to_string()))?;
    self.assert_dc(st, spark_id)?;

    let allow = globset_from_opt_secure(st.env.permissions.dc.allow_config_paths.as_ref())
        .map_err(|e| WorldError::Internal(e.to_string()))?;

    if st.env.permissions.dc.allow_config_paths.is_none() {
      return Err(WorldError::AclViolation("dc_edit_config not enabled".into()));
    }

    let mut edited_paths = vec![];
    for e in edits {
      if !allow.is_match(&e.path) {
        return Err(WorldError::AclViolation(format!("DC cannot edit config path {}", e.path)));
      }

      let abs = secure_join(&st.workspace_path, &e.path)?;
      if let Some(parent) = abs.parent() { fs::create_dir_all(parent).map_err(|e| WorldError::Internal(e.to_string()))?; }
      fs::write(&abs, &e.contents).map_err(|e| WorldError::Internal(e.to_string()))?;

      edited_paths.push(e.path);
    }

    Ok(edited_paths)
  }

  pub async fn apply_policy_violation(&self, run_id: &str, spark_id: &str) -> Option<i32> {
    let mut runs = self.runs.write().await;
    if let Some(st) = runs.get_mut(run_id) {
      let damage = st.env.hp.damage.policy_violation_attempt;
      for s in st.snapshot.sparks.iter_mut() {
        if s.id == spark_id {
          s.hp = (s.hp - damage).max(0);
          return Some(-damage);
        }
      }
    }
    None
  }

}

fn secure_join(base: &str, user_path: &str) -> Result<PathBuf, WorldError> {
    let base_p = Path::new(base);
    let user_p = Path::new(user_path);
    
    if user_p.is_absolute() {
        return Err(WorldError::PathTraversal(format!("Absolute path rejected: {}", user_path)));
    }
    
    let joined = base_p.join(user_p);
    
    for component in user_p.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err(WorldError::PathTraversal(format!("Traversal rejected: {}", user_path)));
        }
    }
    
    Ok(joined)
}

fn opened_gate_id_list_contains(list: &[(String, String)], gate_id: &str) -> bool {
    list.iter().any(|(id, _)| id == gate_id)
}

fn globset_from_opt_secure(globs: Option<&Vec<String>>) -> anyhow::Result<globset::GlobSet> {
  let mut b = GlobSetBuilder::new();
  if let Some(gs) = globs {
    for g in gs {
      b.add(Glob::new(g)?);
    }
  } else {
    b.add(Glob::new("!**")?);
  }
  Ok(b.build()?)
}

fn copy_dir(src: &PathBuf, dst: &PathBuf) -> anyhow::Result<()> {
  use walkdir::WalkDir;
  for entry in WalkDir::new(src) {
    let entry = entry?;
    let rel = entry.path().strip_prefix(src)?;
    let target = dst.join(rel);
    if entry.file_type().is_dir() {
      fs::create_dir_all(&target)?;
    } else {
      if let Some(parent) = target.parent() { fs::create_dir_all(parent)?; }
      fs::copy(entry.path(), &target)?;
    }
  }
  Ok(())
}

fn dedup_nodes(nodes: Vec<BoardNode>) -> Vec<BoardNode> {
  let mut map: std::collections::HashMap<String, BoardNode> = std::collections::HashMap::new();
  for n in nodes { map.insert(n.id.clone(), n); }
  map.into_values().collect()
}
