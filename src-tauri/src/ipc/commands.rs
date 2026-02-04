use serde::{Deserialize, Serialize};
use tauri::Emitter;

use crate::ipc::events::EventEnvelope;
use crate::schemas::environment::{BoardDiff, BoardSnapshot, VisibleSlice, SignalMsg, SparkState};
use crate::schemas::construct::{ConstructSpec, ConstructSpecWrapper, EvaluateResult, ConstructSpecRef};
use crate::core::world::WorldError;
use crate::core::constructs::EvaluationEngine;

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
  pub code: String,
  pub message: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub detail: Option<serde_json::Value>,
}

impl ApiError {
    pub fn new(code: &str, message: &str) -> Self {
        Self { code: code.to_string(), message: message.to_string(), detail: None }
    }
    
    pub fn from_world(err: WorldError) -> Self {
        match err {
            WorldError::AclViolation(m) => Self { code: "ACL_VIOLATION".into(), message: m, detail: None },
            WorldError::NotFound(m) => Self { code: "NOT_FOUND".into(), message: m, detail: None },
            WorldError::InvalidRequest(m) => Self { code: "INVALID".into(), message: m, detail: None },
            WorldError::Internal(m) => Self { code: "INTERNAL".into(), message: m, detail: None },
            WorldError::PathTraversal(m) => Self { code: "SECURITY_VIOLATION".into(), message: m, detail: None },
        }
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

fn now_ms_u64() -> u64 {
  use std::time::{SystemTime, UNIX_EPOCH};
  SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(std::time::Duration::from_secs(0)).as_millis() as u64
}

/// Fire-and-forget emit with error logging for observability
fn emit_event<T: Serialize + Clone>(app: &tauri::AppHandle, event: &str, payload: T) {
  if let Err(e) = app.emit(event, payload) {
    eprintln!("EVENT EMIT FAILURE: Failed to emit '{}': {}", event, e);
  }
}

// ---------- Run lifecycle ----------
#[derive(Debug, Serialize, Deserialize)]
pub struct RunCreateReq { pub env_spec_path: String, pub seed: Option<u64> }

#[derive(Debug, Serialize, Deserialize)]
pub struct RunCreateRes { pub run_id: String, pub workspace_path: String, pub board: BoardSnapshot }

#[tauri::command]
pub async fn run_create(app: tauri::AppHandle, state: tauri::State<'_, crate::ipc::AppState>, req: RunCreateReq) -> ApiResult<RunCreateRes> {
  let run_id = uuid::Uuid::new_v4().to_string();
  let (workspace_path, snapshot, _seq) = state.world.init_run(&run_id, &req.env_spec_path, req.seed).await
      .map_err(|e| ApiError::new("RUN_CREATE_FAILED", &e.to_string()))?;

  state.storage.event_log.create_run_row(&run_id, &req.env_spec_path, &workspace_path, req.seed).await
    .map_err(|e| ApiError::new("DB", &e.to_string()))?;

  // DO NOT IGNORE DB ERRORS FOR AUDIT LOGS
  state.storage.event_log.append_event(&run_id, "RunCreated", serde_json::json!({
    "envSpecPath": req.env_spec_path,
    "workspacePath": workspace_path,
    "seed": req.seed
  })).await.map_err(|e| ApiError::new("DB", &e.to_string()))?;

  let snapshot_val = serde_json::to_value(&snapshot).map_err(|e| ApiError::new("SERIALIZE", &e.to_string()))?;
  let seq_snapshot = state.storage.event_log.append_event(&run_id, "BoardSnapshotIssued", snapshot_val).await
    .map_err(|e| ApiError::new("DB", &e.to_string()))?;

  let env = EventEnvelope {
    run_id: run_id.clone(),
    seq: seq_snapshot,
    ts: now_ms_u64(),
    r#type: "board.snapshot".into(),
    payload: snapshot.clone(),
  };
  emit_event(&app, "board.snapshot", env);

  Ok(RunCreateRes { run_id, workspace_path, board: snapshot })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RunLoadReq { pub run_id: String }

#[derive(Debug, Serialize, Deserialize)]
pub struct RunLoadRes { pub run_id: String, pub board: BoardSnapshot }

#[tauri::command]
pub async fn run_load(app: tauri::AppHandle, state: tauri::State<'_, crate::ipc::AppState>, req: RunLoadReq) -> ApiResult<RunLoadRes> {
  let snapshot = state.world.load_run(&state.storage, &req.run_id).await
      .map_err(|e| ApiError::new("RUN_LOAD_FAILED", &e.to_string()))?;
      
  let env = EventEnvelope { run_id: req.run_id.clone(), seq: 0, ts: now_ms_u64(), r#type: "board.snapshot".into(), payload: snapshot.clone() };
  emit_event(&app, "board.snapshot", env);
  Ok(RunLoadRes { run_id: req.run_id, board: snapshot })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RunResetReq { pub run_id: String }

#[tauri::command]
pub async fn run_reset_to_checkpoint(app: tauri::AppHandle, state: tauri::State<'_, crate::ipc::AppState>, req: RunResetReq) -> ApiResult<()> {
  state.world.clear_run(&req.run_id).await;

  let snapshot = state.world.load_run(&state.storage, &req.run_id).await
      .map_err(|e| ApiError::new("RESET_FAILED", &e.to_string()))?;

  let seq = state.storage.event_log.append_event(&req.run_id, "CheckpointReset", serde_json::json!({ "runId": req.run_id })).await
    .map_err(|e| ApiError::new("DB", &e.to_string()))?;

  let env = EventEnvelope { run_id: req.run_id.clone(), seq, ts: now_ms_u64(), r#type: "board.snapshot".into(), payload: snapshot };
  emit_event(&app, "board.snapshot", env);

  Ok(())
}

// ---------- Generation ----------
#[derive(Debug, Serialize, Deserialize)]
pub struct SparkSpec { pub id: String, pub kind: String }

#[derive(Debug, Serialize, Deserialize)]
pub struct TeamSpec { pub sparks: Vec<SparkSpec>, pub dc_config_mode: String }

#[derive(Debug, Serialize, Deserialize)]
pub struct CompileProblemReq {
  pub prompt: String,
  pub language_profile: String,
  pub difficulty: String,
  pub team: TeamSpec,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompileProblemRes { pub problem_spec_path: String }

#[tauri::command]
pub async fn gen_compile_problem(state: tauri::State<'_, crate::ipc::AppState>, req: CompileProblemReq) -> ApiResult<CompileProblemRes> {
  let path = state.generator.save_problem_spec(&req).await.map_err(|e| ApiError::new("GEN", &e.to_string()))?;
  Ok(CompileProblemRes { problem_spec_path: path })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateEnvironmentReq { pub problem_spec_path: String, pub out_dir: String }

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateEnvironmentRes { pub env_spec_path: String, pub repo_template_path: String }

#[tauri::command]
pub async fn gen_generate_environment(state: tauri::State<'_, crate::ipc::AppState>, req: GenerateEnvironmentReq) -> ApiResult<GenerateEnvironmentRes> {
  let (env_spec_path, repo_template_path) = state.generator.scaffold_environment(&req.problem_spec_path, &req.out_dir)
    .await.map_err(|e| ApiError::new("GEN", &e.to_string()))?;
  Ok(GenerateEnvironmentRes { env_spec_path, repo_template_path })
}

// ---------- World observe/act ----------
#[derive(Debug, Serialize, Deserialize)]
pub struct ObserveReq { pub run_id: String, pub spark_id: String }

#[derive(Debug, Serialize, Deserialize)]
pub struct ObserveRes { pub spark: SparkState, pub visible: VisibleSlice, pub inbox: Vec<SignalMsg> }

#[tauri::command]
pub async fn world_observe(state: tauri::State<'_, crate::ipc::AppState>, req: ObserveReq) -> ApiResult<ObserveRes> {
  let (spark, visible, inbox) = state.world.observe(&req.run_id, &req.spark_id)
    .await.map_err(|e| ApiError::new("OBSERVE", &e.to_string()))?;
  Ok(ObserveRes { spark, visible, inbox })
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WorldAction {
  #[serde(rename="move")] Move { run_id: String, spark_id: String, to_node_id: String },
  #[serde(rename="energize")] Energize { run_id: String, spark_id: String, object_id: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActReq { pub action: WorldAction }

#[derive(Debug, Serialize, Deserialize)]
pub struct ActRes { pub diff: Option<BoardDiff> }

#[tauri::command]
pub async fn world_act(app: tauri::AppHandle, state: tauri::State<'_, crate::ipc::AppState>, req: ActReq) -> ApiResult<ActRes> {
  let outcome = state.world.act(&state.storage, req.action).await.map_err(ApiError::from_world)?;

  let env = EventEnvelope { run_id: outcome.run_id.clone(), seq: outcome.seq, ts: now_ms_u64(), r#type: "board.diff".into(), payload: outcome.diff.clone() };
  emit_event(&app, "board.diff", env);

  Ok(ActRes { diff: Some(outcome.diff) })
}

// ---------- AC surface ----------
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEdit { pub path: String, pub contents: String }

#[derive(Debug, Serialize, Deserialize)]
pub struct EditFilesReq { pub run_id: String, pub spark_id: String, pub edits: Vec<FileEdit> }

#[tauri::command]
pub async fn ac_edit_files(state: tauri::State<'_, crate::ipc::AppState>, req: EditFilesReq) -> ApiResult<()> {
  let edits_copy = req.edits.clone();
  let result = state.world.ac_edit_files(&req.run_id, &req.spark_id, req.edits).await;

  match result {
    Ok(()) => {
      let paths: Vec<String> = edits_copy.into_iter().map(|e| e.path).collect();
      state.storage.event_log.append_event(&req.run_id, "FilesEdited", serde_json::json!({
        "sparkId": req.spark_id,
        "touchedPaths": paths,
      })).await.map_err(|e| ApiError::new("DB", &e.to_string()))?;
      Ok(())
    },
    Err(e) => {
      let msg = e.to_string();
      if matches!(e, WorldError::AclViolation(_)) {
        // MUST CHECK RESULTS FOR SECURITY SIDE EFFECTS
        let hp_delta = state.world.apply_policy_violation(&req.run_id, &req.spark_id).await;
        if let Err(db_err) = state.storage.event_log.append_event(&req.run_id, "PolicyViolation", serde_json::json!({
          "sparkId": req.spark_id,
          "action": "ac_edit_files",
          "reason": msg,
          "hpDelta": hp_delta,
        })).await {
          eprintln!("SECURITY AUDIT FAILURE: Could not persist PolicyViolation event: {}", db_err);
        }
      }
      Err(ApiError::from_world(e))
    }
  }
}

// ---------- Constructs ----------
#[derive(Debug, Serialize, Deserialize)]
pub struct ConstructDefineReq { pub run_id: String, pub spec: ConstructSpecWrapper }

#[derive(Debug, Serialize, Deserialize)]
pub struct ConstructDefineRes { pub construct_id: String }

#[tauri::command]
pub async fn construct_define(state: tauri::State<'_, crate::ipc::AppState>, req: ConstructDefineReq) -> ApiResult<ConstructDefineRes> {
    let spec = req.spec.construct;
    let construct_id = state.storage.constructs.create(&req.run_id, spec.kind.clone(), &spec.name, &spec.root).await
        .map_err(|e| ApiError::new("DB", &e.to_string()))?;
    
    let (_revision, spec_hash) = state.storage.constructs.add_spec(&construct_id, &spec).await
        .map_err(|e| ApiError::new("DB", &e.to_string()))?;
    
    state.storage.event_log.append_event(&req.run_id, "ConstructDefined", serde_json::json!({
        "constructId": construct_id,
        "name": spec.name,
        "kind": spec.kind,
        "specHash": spec_hash,
    })).await.map_err(|e| ApiError::new("DB", &e.to_string()))?;

    Ok(ConstructDefineRes { construct_id })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConstructUpdateSpecReq { pub construct_id: String, pub spec: ConstructSpecWrapper, pub run_id: String }

#[tauri::command]
pub async fn construct_update_spec(state: tauri::State<'_, crate::ipc::AppState>, req: ConstructUpdateSpecReq) -> ApiResult<i64> {
    let spec = req.spec.construct;
    let (revision, spec_hash) = state.storage.constructs.add_spec(&req.construct_id, &spec).await
        .map_err(|e| ApiError::new("DB", &e.to_string()))?;
    
    state.storage.event_log.append_event(&req.run_id, "ConstructSpecUpdated", serde_json::json!({
        "constructId": req.construct_id,
        "revision": revision,
        "specHash": spec_hash,
    })).await.map_err(|e| ApiError::new("DB", &e.to_string()))?;

    Ok(revision)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConstructGetReq { pub construct_id: String }

#[derive(Debug, Serialize, Deserialize)]
pub struct ConstructGetRes { pub spec_ref: ConstructSpecRef }

#[tauri::command]
pub async fn construct_get(state: tauri::State<'_, crate::ipc::AppState>, req: ConstructGetReq) -> ApiResult<ConstructGetRes> {
    let spec_ref = state.storage.constructs.get_latest_spec(&req.construct_id).await
        .map_err(|e| ApiError::new("DB", &e.to_string()))?
        .ok_or_else(|| ApiError::new("NOT_FOUND", "Construct not found"))?;
    
    Ok(ConstructGetRes { spec_ref })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConstructEvaluateReq { pub run_id: String, pub construct_id: String }

#[tauri::command]
pub async fn construct_evaluate(state: tauri::State<'_, crate::ipc::AppState>, req: ConstructEvaluateReq) -> ApiResult<EvaluateResult> {
    let res = EvaluationEngine::evaluate(&state.storage, &req.run_id, &req.construct_id).await
        .map_err(|e| ApiError::new("EVAL", &e.to_string()))?;
    
    Ok(res)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConstructListReq { pub run_id: String }

#[tauri::command]
pub async fn construct_list(state: tauri::State<'_, crate::ipc::AppState>, req: ConstructListReq) -> ApiResult<Vec<String>> {
    let list = state.storage.constructs.list_for_run(&req.run_id).await
        .map_err(|e| ApiError::new("DB", &e.to_string()))?;
    
    Ok(list.into_iter().map(|c| c.construct_id).collect())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePatchReq {
  pub run_id: String,
  pub spark_id: String,
  pub message: String,
  pub intent: Option<String>,
  pub risk: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePatchRes { pub patch_id: String, pub item_id: String, pub touched_paths: Vec<String> }

#[tauri::command]
pub async fn ac_create_patch(state: tauri::State<'_, crate::ipc::AppState>, req: CreatePatchReq) -> ApiResult<CreatePatchRes> {
  let res = state.world.ac_create_patch(&state.storage, &req.run_id, &req.spark_id, &req.message).await
    .map_err(|e| ApiError::new("AC_PATCH", &e.to_string()))?;

  state.storage.event_log.append_event(&req.run_id, "PatchCreated", serde_json::json!({
    "sparkId": req.spark_id,
    "patchId": res.patch_id,
    "itemId": res.item_id,
    "touchedPaths": res.touched_paths,
    "message": req.message,
  })).await.map_err(|e| ApiError::new("DB", &e.to_string()))?;

  Ok(CreatePatchRes { patch_id: res.patch_id, item_id: res.item_id, touched_paths: res.touched_paths })
}

// ---------- DC surface ----------
#[derive(Debug, Serialize, Deserialize)]
pub struct ApplyPatchReq { pub run_id: String, pub spark_id: String, pub patch_id: String }

#[tauri::command]
pub async fn dc_apply_patch(state: tauri::State<'_, crate::ipc::AppState>, req: ApplyPatchReq) -> ApiResult<()> {
  let touched_paths = state.world.dc_apply_patch(&state.storage, &req.run_id, &req.spark_id, &req.patch_id).await
    .map_err(|e| ApiError::new("DC_APPLY", &e.to_string()))?;

  state.storage.event_log.append_event(&req.run_id, "PatchApplied", serde_json::json!({
    "sparkId": req.spark_id,
    "patchId": req.patch_id,
    "touchedPaths": touched_paths,
  })).await.map_err(|e| ApiError::new("DB", &e.to_string()))?;

  Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RunProbeReq { pub run_id: String, pub spark_id: String, pub probe_id: String }

#[derive(Debug, Serialize, Deserialize)]
pub struct RunProbeRes {
  pub probe_id: String,
  pub status: String,
  pub evidence_item_ids: Vec<String>,
  pub artifact_refs: Vec<String>,
  pub failure_signature: Option<String>,
}

#[tauri::command]
pub async fn dc_run_probe(app: tauri::AppHandle, state: tauri::State<'_, crate::ipc::AppState>, req: RunProbeReq) -> ApiResult<RunProbeRes> {
  // ProbeFinished event is now persisted inside dc_run_probe (persist-first pattern)
  let outcome = state.world.dc_run_probe(&state.storage, &state.sandbox, &req.run_id, &req.spark_id, &req.probe_id).await
      .map_err(|e| ApiError::new("DC_PROBE", &e.to_string()))?;

  let env = EventEnvelope { run_id: req.run_id.clone(), seq: outcome.seq, ts: now_ms_u64(), r#type: "board.diff".into(), payload: outcome.diff.clone() };
  emit_event(&app, "board.diff", env);

  Ok(RunProbeRes {
    probe_id: req.probe_id,
    status: outcome.status,
    evidence_item_ids: outcome.evidence_item_ids,
    artifact_refs: outcome.artifact_refs,
    failure_signature: outcome.failure_signature,
  })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitChecklistReq { pub run_id: String, pub spark_id: String, pub relay_id: String, pub checklist: serde_json::Value }

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitChecklistRes { pub checklist_id: String }

#[tauri::command]
pub async fn dc_submit_checklist(state: tauri::State<'_, crate::ipc::AppState>, req: SubmitChecklistReq) -> ApiResult<SubmitChecklistRes> {
  let checklist_bytes = serde_json::to_vec_pretty(&req.checklist)
    .map_err(|e| ApiError::new("SERIALIZE", &e.to_string()))?;

  let artifact_ref = state.storage.artifacts.put_bytes(
    &req.run_id,
    "checklist",
    Some("application/json"),
    Some(&format!("checklists/{}/{}_{}", req.relay_id, chrono::Utc::now().timestamp_millis(), uuid::Uuid::new_v4())),
    &checklist_bytes,
    Some(serde_json::json!({ "sparkId": req.spark_id, "relayId": req.relay_id })),
  ).await.map_err(|e| ApiError::new("ARTIFACT", &e.to_string()))?;

  state.storage.event_log.append_event(&req.run_id, "ChecklistSubmitted", serde_json::json!({
    "sparkId": req.spark_id,
    "relayId": req.relay_id,
    "checklistId": artifact_ref.artifact_id,
  })).await.map_err(|e| ApiError::new("DB", &e.to_string()))?;

  Ok(SubmitChecklistRes { checklist_id: artifact_ref.artifact_id })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CloseRelayReq { pub run_id: String, pub spark_id: String, pub relay_id: String }

#[tauri::command]
pub async fn dc_close_relay(app: tauri::AppHandle, state: tauri::State<'_, crate::ipc::AppState>, req: CloseRelayReq) -> ApiResult<()> {
  // RelayClosed event is now persisted inside dc_close_relay (persist-first pattern)
  let outcome = state.world.dc_close_relay(&state.storage, &req.run_id, &req.spark_id, &req.relay_id).await
    .map_err(|e| ApiError::new("DC_RELAY", &e.to_string()))?;

  let env = EventEnvelope { run_id: req.run_id.clone(), seq: outcome.seq, ts: now_ms_u64(), r#type: "board.diff".into(), payload: outcome.diff.clone() };
  emit_event(&app, "board.diff", env);

  Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DcEditConfigReq { pub run_id: String, pub spark_id: String, pub reason: String, pub edits: Vec<FileEdit> }

#[derive(Debug, Serialize, Deserialize)]
pub struct DcEditConfigRes { pub edited_paths: Vec<String> }

#[tauri::command]
pub async fn dc_edit_config(state: tauri::State<'_, crate::ipc::AppState>, req: DcEditConfigReq) -> ApiResult<DcEditConfigRes> {
  let result = state.world.dc_edit_config(&req.run_id, &req.spark_id, &req.reason, req.edits.clone()).await;

  match result {
    Ok(edited_paths) => {
      state.storage.event_log.append_event(&req.run_id, "ConfigEdited", serde_json::json!({
        "sparkId": req.spark_id,
        "reason": req.reason,
        "touchedPaths": edited_paths,
      })).await.map_err(|e| ApiError::new("DB", &e.to_string()))?;
      Ok(DcEditConfigRes { edited_paths })
    }
    Err(e) => {
      let msg = e.to_string();
      if matches!(e, WorldError::AclViolation(_)) {
        let hp_delta = state.world.apply_policy_violation(&req.run_id, &req.spark_id).await;
        if let Err(db_err) = state.storage.event_log.append_event(&req.run_id, "PolicyViolation", serde_json::json!({
          "sparkId": req.spark_id,
          "action": "dc_edit_config",
          "reason": msg,
          "hpDelta": hp_delta,
        })).await {
          eprintln!("SECURITY AUDIT FAILURE: Could not persist PolicyViolation event: {}", db_err);
        }
      }
      Err(ApiError::from_world(e))
    }
  }
}

// ---------- HITL Signoff ----------
#[derive(Debug, Serialize, Deserialize)]
pub struct HitlChecks {
  pub reality_contact: bool,
  pub distance_reduction: bool,
  pub anti_goodhart: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HitlSignoffWriteReq {
  pub run_id: String,
  pub construct_root: String,
  pub cycle_day: String,
  pub decision: String, // "approved" or "rejected"
  pub checks: HitlChecks,
  pub notes: Option<String>,
  pub artifacts_reviewed: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HitlSignoffWriteRes {
  pub written_path: String,
  pub capsule_hash: String,
}

/// Writes HITL evidence file with capsule hash binding.
///
/// Security: Only writes to allowlisted paths (**/evidence/hitl*.json)
#[tauri::command]
pub async fn hitl_signoff_write(state: tauri::State<'_, crate::ipc::AppState>, req: HitlSignoffWriteReq) -> ApiResult<HitlSignoffWriteRes> {
  use sha2::{Sha256, Digest};
  use std::path::PathBuf;

  // Get workspace path for this run
  let workspace_path = state.world.get_workspace_path(&req.run_id).await
    .map_err(|e| ApiError::new("WORKSPACE", &e.to_string()))?;

  // Construct the HITL evidence path
  let evidence_path = PathBuf::from(&workspace_path)
    .join(&req.construct_root)
    .join("days")
    .join(&req.cycle_day)
    .join("evidence")
    .join("hitl_weekly.json");

  // Security: Validate path is within allowed locations
  let canonical_path = evidence_path.canonicalize().unwrap_or(evidence_path.clone());
  let canonical_workspace = PathBuf::from(&workspace_path).canonicalize()
    .map_err(|e| ApiError::new("PATH", &e.to_string()))?;

  if !canonical_path.starts_with(&canonical_workspace) {
    return Err(ApiError::new("SECURITY_VIOLATION", "Path traversal attempt detected"));
  }

  // Validate filename pattern (allowlist: hitl*.json)
  let filename = evidence_path.file_name()
    .and_then(|n| n.to_str())
    .ok_or_else(|| ApiError::new("PATH", "Invalid filename"))?;

  if !filename.starts_with("hitl") || !filename.ends_with(".json") {
    return Err(ApiError::new("SECURITY_VIOLATION", "HITL writes restricted to hitl*.json files"));
  }

  // Compute capsule hash from artifacts
  let mut hasher = Sha256::new();
  let base_path = evidence_path.parent()
    .and_then(|p| p.parent())
    .ok_or_else(|| ApiError::new("PATH", "Invalid evidence path structure"))?;

  let mut sorted_artifacts = req.artifacts_reviewed.clone();
  sorted_artifacts.sort();

  for artifact in &sorted_artifacts {
    let artifact_path = base_path.join(artifact);
    if artifact_path.exists() {
      let content = std::fs::read(&artifact_path)
        .map_err(|e| ApiError::new("IO", &format!("Cannot read {}: {}", artifact, e)))?;
      hasher.update(artifact.as_bytes());
      hasher.update(&content);
    }
  }

  let capsule_hash = format!("{:x}", hasher.finalize());

  // Build the HITL evidence JSON
  let hitl_evidence = serde_json::json!({
    "version": "2.1",
    "kind": "hitl_weekly_audit",
    "reviewer": "human",
    "decision": req.decision,
    "timestamp_utc": chrono::Utc::now().to_rfc3339(),
    "checks": {
      "reality_contact": req.checks.reality_contact,
      "distance_reduction": req.checks.distance_reduction,
      "anti_goodhart": req.checks.anti_goodhart,
    },
    "notes": req.notes,
    "artifacts_reviewed": req.artifacts_reviewed,
    "capsule_hash": capsule_hash,
  });

  // Ensure evidence directory exists
  if let Some(parent) = evidence_path.parent() {
    std::fs::create_dir_all(parent)
      .map_err(|e| ApiError::new("IO", &format!("Cannot create evidence directory: {}", e)))?;
  }

  // Write the file
  let json_str = serde_json::to_string_pretty(&hitl_evidence)
    .map_err(|e| ApiError::new("SERIALIZE", &e.to_string()))?;

  std::fs::write(&evidence_path, &json_str)
    .map_err(|e| ApiError::new("IO", &format!("Cannot write HITL evidence: {}", e)))?;

  // Persist event
  state.storage.event_log.append_event(&req.run_id, "HitlSignedOff", serde_json::json!({
    "reviewer": "human",
    "decision": req.decision,
    "cycleDay": req.cycle_day,
    "capsuleHash": capsule_hash,
    "path": evidence_path.to_string_lossy(),
  })).await.map_err(|e| ApiError::new("DB", &e.to_string()))?;

  Ok(HitlSignoffWriteRes {
    written_path: evidence_path.to_string_lossy().to_string(),
    capsule_hash,
  })
}