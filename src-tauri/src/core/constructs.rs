use std::collections::HashMap;
use crate::schemas::construct::*;
use crate::storage::Storage;
use anyhow::Context;

pub struct EvaluationEngine;

impl EvaluationEngine {
    pub async fn evaluate(storage: &Storage, run_id: &str, construct_id: &str) -> anyhow::Result<EvaluateResult> {
        let spec_ref = storage.constructs.get_latest_spec(construct_id).await?
            .context("Construct spec not found")?;
        let spec = spec_ref.spec;
        
        let events = storage.event_log.read_all_events(run_id).await?;
        
        let mut last_mutation_seq = 0u64;
        let mut last_probe_results: HashMap<String, (String, u64)> = HashMap::new(); // probe_id -> (status, seq)

        for (seq, event_type, payload) in events {
            if event_type == "PatchApplied" {
                if let Some(touched) = payload.get("touchedPaths").and_then(|t| t.as_array()) {
                    let any_under_root = touched.iter().any(|p| {
                        p.as_str().map(|path| path.starts_with(&spec.root)).unwrap_or(false)
                    });
                    if any_under_root {
                        last_mutation_seq = seq;
                    }
                }
            } else if event_type == "ProbeFinished" {
                if let (Some(pid), Some(status)) = (
                    payload.get("probeId").and_then(|p| p.as_str()),
                    payload.get("status").and_then(|s| s.as_str())
                ) {
                    last_probe_results.insert(pid.to_string(), (status.to_string(), seq));
                }
            }
        }

        let mut satisfied_invariants = vec![];
        let mut failing_probes = vec![];
        let mut missing_probes = vec![];

        // Check acceptance probes for certification
        for probe_ref in &spec.acceptance.required_probes {
            let probe_id = probe_ref.strip_prefix("probe:").unwrap_or(probe_ref);
            if let Some((status, seq)) = last_probe_results.get(probe_id) {
                if *seq > last_mutation_seq {
                    if status == "pass" {
                        // pass
                    } else {
                        failing_probes.push(probe_id.to_string());
                    }
                } else {
                    missing_probes.push(probe_id.to_string());
                }
            } else {
                missing_probes.push(probe_id.to_string());
            }
        }

        // Check all invariants for satisfaction
        for inv in &spec.invariants {
            let probe_id = inv.probe_ref.strip_prefix("probe:").unwrap_or(&inv.probe_ref);
            if let Some((status, seq)) = last_probe_results.get(probe_id) {
                 if *seq > last_mutation_seq && status == "pass" {
                     satisfied_invariants.push(inv.id.clone());
                 }
            }
        }

        // Determine status based on probes and mutations
        let status = if !failing_probes.is_empty() {
            ConstructStatus::Active // In v1 we don't track "ever certified" for Regressed yet
        } else if missing_probes.is_empty() {
            ConstructStatus::Certified
        } else if last_mutation_seq > 0 || !last_probe_results.is_empty() {
            ConstructStatus::Active
        } else {
            ConstructStatus::Draft
        };

        Ok(EvaluateResult {
            status,
            missing_probes,
            failing_probes,
            satisfied_invariants,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_evaluate_draft() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("test.sqlite");
        let artifact_root = dir.path().join("artifacts");
        let storage = Storage::new(db_path.to_str().unwrap(), artifact_root.to_str().unwrap()).await?;
        
        let run_id = "test_run";
        storage.event_log.create_run_row(run_id, "spec.yaml", "workspace", None).await?;
        
        let spec = ConstructSpec {
            id: "c1".into(),
            kind: ConstructKind::Cli,
            name: "Test".into(),
            description: "Desc".into(),
            root: "src".into(),
            interface: Interface { exports: vec![] },
            invariants: vec![],
            acceptance: Acceptance { required_probes: vec!["probe:p1".into()], pass_policy: PassPolicy { mode: PassPolicyMode::All } },
            dependencies: None,
            metadata: None,
        };
        
        let construct_id = storage.constructs.create(run_id, spec.kind.clone(), &spec.name, &spec.root).await?;
        storage.constructs.add_spec(&construct_id, &spec).await?;
        
        let res = EvaluationEngine::evaluate(&storage, run_id, &construct_id).await?;
        assert_eq!(res.status, ConstructStatus::Draft);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_evaluate_certified() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("test.sqlite");
        let artifact_root = dir.path().join("artifacts");
        let storage = Storage::new(db_path.to_str().unwrap(), artifact_root.to_str().unwrap()).await?;
        
        let run_id = "test_run";
        storage.event_log.create_run_row(run_id, "spec.yaml", "workspace", None).await?;
        
        let spec = ConstructSpec {
            id: "c1".into(),
            kind: ConstructKind::Cli,
            name: "Test".into(),
            description: "Desc".into(),
            root: "src".into(),
            interface: Interface { exports: vec![] },
            invariants: vec![],
            acceptance: Acceptance { required_probes: vec!["probe:p1".into()], pass_policy: PassPolicy { mode: PassPolicyMode::All } },
            dependencies: None,
            metadata: None,
        };
        
        let construct_id = storage.constructs.create(run_id, spec.kind.clone(), &spec.name, &spec.root).await?;
        storage.constructs.add_spec(&construct_id, &spec).await?;
        
        // 1. Mutation
        storage.event_log.append_event(run_id, "PatchApplied", serde_json::json!({
            "touchedPaths": ["src/main.py"]
        })).await?;
        
        // 2. Probe passes
        storage.event_log.append_event(run_id, "ProbeFinished", serde_json::json!({
            "probeId": "p1",
            "status": "pass"
        })).await?;
        
        let res = EvaluationEngine::evaluate(&storage, run_id, &construct_id).await?;
        assert_eq!(res.status, ConstructStatus::Certified);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_evaluate_active_due_to_stale_probe() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("test.sqlite");
        let artifact_root = dir.path().join("artifacts");
        let storage = Storage::new(db_path.to_str().unwrap(), artifact_root.to_str().unwrap()).await?;
        
        let run_id = "test_run";
        storage.event_log.create_run_row(run_id, "spec.yaml", "workspace", None).await?;
        
        let spec = ConstructSpec {
            id: "c1".into(),
            kind: ConstructKind::Cli,
            name: "Test".into(),
            description: "Desc".into(),
            root: "src".into(),
            interface: Interface { exports: vec![] },
            invariants: vec![],
            acceptance: Acceptance { required_probes: vec!["probe:p1".into()], pass_policy: PassPolicy { mode: PassPolicyMode::All } },
            dependencies: None,
            metadata: None,
        };
        
        let construct_id = storage.constructs.create(run_id, spec.kind.clone(), &spec.name, &spec.root).await?;
        storage.constructs.add_spec(&construct_id, &spec).await?;
        
        // 1. Probe passes FIRST
        storage.event_log.append_event(run_id, "ProbeFinished", serde_json::json!({
            "probeId": "p1",
            "status": "pass"
        })).await?;

        // 2. THEN Mutation occurs
        storage.event_log.append_event(run_id, "PatchApplied", serde_json::json!({
            "touchedPaths": ["src/main.py"]
        })).await?;
        
        let res = EvaluationEngine::evaluate(&storage, run_id, &construct_id).await?;
        assert_eq!(res.status, ConstructStatus::Active);
        assert_eq!(res.missing_probes, vec!["p1".to_string()]);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_evaluate_mutation_invalidates_certification() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("test.sqlite");
        let artifact_root = dir.path().join("artifacts");
        let storage = Storage::new(db_path.to_str().unwrap(), artifact_root.to_str().unwrap()).await?;
        
        let run_id = "test_run";
        storage.event_log.create_run_row(run_id, "spec.yaml", "workspace", None).await?;
        
        let spec = ConstructSpec {
            id: "c1".into(),
            kind: ConstructKind::Cli,
            name: "Test".into(),
            description: "Desc".into(),
            root: "src".into(),
            interface: Interface { exports: vec![] },
            invariants: vec![],
            acceptance: Acceptance { required_probes: vec!["probe:p1".into()], pass_policy: PassPolicy { mode: PassPolicyMode::All } },
            dependencies: None,
            metadata: None,
        };
        
        let construct_id = storage.constructs.create(run_id, spec.kind.clone(), &spec.name, &spec.root).await?;
        storage.constructs.add_spec(&construct_id, &spec).await?;
        
        // 1. Mutation
        storage.event_log.append_event(run_id, "PatchApplied", serde_json::json!({
            "touchedPaths": ["src/main.py"]
        })).await?;
        
        // 2. Probe passes
        storage.event_log.append_event(run_id, "ProbeFinished", serde_json::json!({
            "probeId": "p1",
            "status": "pass"
        })).await?;
        
        // 3. Certified
        let res = EvaluationEngine::evaluate(&storage, run_id, &construct_id).await?;
        assert_eq!(res.status, ConstructStatus::Certified);

        // 4. NEW Mutation
        storage.event_log.append_event(run_id, "PatchApplied", serde_json::json!({
            "touchedPaths": ["src/logic.py"]
        })).await?;

        // 5. Regressed to Active (in v1)
        let res2 = EvaluationEngine::evaluate(&storage, run_id, &construct_id).await?;
        assert_eq!(res2.status, ConstructStatus::Active);
        assert_eq!(res2.missing_probes, vec!["p1".to_string()]);

        // 6. Re-run probe
        storage.event_log.append_event(run_id, "ProbeFinished", serde_json::json!({
            "probeId": "p1",
            "status": "pass"
        })).await?;

        // 7. Certified again
        let res3 = EvaluationEngine::evaluate(&storage, run_id, &construct_id).await?;
        assert_eq!(res3.status, ConstructStatus::Certified);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_evaluate_mutation_outside_root_does_not_invalidate() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("test.sqlite");
        let artifact_root = dir.path().join("artifacts");
        let storage = Storage::new(db_path.to_str().unwrap(), artifact_root.to_str().unwrap()).await?;
        
        let run_id = "test_run";
        storage.event_log.create_run_row(run_id, "spec.yaml", "workspace", None).await?;
        
        let spec = ConstructSpec {
            id: "c1".into(),
            kind: ConstructKind::Cli,
            name: "Test".into(),
            description: "Desc".into(),
            root: "src".into(),
            interface: Interface { exports: vec![] },
            invariants: vec![],
            acceptance: Acceptance { required_probes: vec!["probe:p1".into()], pass_policy: PassPolicy { mode: PassPolicyMode::All } },
            dependencies: None,
            metadata: None,
        };
        
        let construct_id = storage.constructs.create(run_id, spec.kind.clone(), &spec.name, &spec.root).await?;
        storage.constructs.add_spec(&construct_id, &spec).await?;
        
        // 1. Mutation inside root
        storage.event_log.append_event(run_id, "PatchApplied", serde_json::json!({
            "touchedPaths": ["src/main.py"]
        })).await?;
        
        // 2. Probe passes
        storage.event_log.append_event(run_id, "ProbeFinished", serde_json::json!({
            "probeId": "p1",
            "status": "pass"
        })).await?;
        
        // 3. Certified
        let res = EvaluationEngine::evaluate(&storage, run_id, &construct_id).await?;
        assert_eq!(res.status, ConstructStatus::Certified);

        // 4. Mutation OUTSIDE root (e.g. docs)
        storage.event_log.append_event(run_id, "PatchApplied", serde_json::json!({
            "touchedPaths": ["docs/README.md"]
        })).await?;

        // 5. Still Certified
        let res2 = EvaluationEngine::evaluate(&storage, run_id, &construct_id).await?;
        assert_eq!(res2.status, ConstructStatus::Certified);
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_staged_events_do_not_invalidate() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("test.sqlite");
        let artifact_root = dir.path().join("artifacts");
        let storage = Storage::new(db_path.to_str().unwrap(), artifact_root.to_str().unwrap()).await?;
        
        let run_id = "test_run";
        storage.event_log.create_run_row(run_id, "spec.yaml", "workspace", None).await?;
        
        let spec = ConstructSpec {
            id: "c1".into(),
            kind: ConstructKind::Cli,
            name: "Test".into(),
            description: "Desc".into(),
            root: "src".into(),
            interface: Interface { exports: vec![] },
            invariants: vec![],
            acceptance: Acceptance { required_probes: vec!["probe:p1".into()], pass_policy: PassPolicy { mode: PassPolicyMode::All } },
            dependencies: None,
            metadata: None,
        };
        
        let construct_id = storage.constructs.create(run_id, spec.kind.clone(), &spec.name, &spec.root).await?;
        storage.constructs.add_spec(&construct_id, &spec).await?;
        
        // 1. Mutation inside root
        storage.event_log.append_event(run_id, "PatchApplied", serde_json::json!({
            "touchedPaths": ["src/main.py"]
        })).await?;
        
        // 2. Probe passes
        storage.event_log.append_event(run_id, "ProbeFinished", serde_json::json!({
            "probeId": "p1",
            "status": "pass"
        })).await?;
        
        // 3. Certified
        let res = EvaluationEngine::evaluate(&storage, run_id, &construct_id).await?;
        assert_eq!(res.status, ConstructStatus::Certified);

        // 4. Staged events (FilesEdited, PatchCreated)
        storage.event_log.append_event(run_id, "FilesEdited", serde_json::json!({
            "touchedPaths": ["src/main.py"]
        })).await?;
        storage.event_log.append_event(run_id, "PatchCreated", serde_json::json!({
            "touchedPaths": ["src/main.py"]
        })).await?;

        // 5. Still Certified (staging doesn't invalidate)
        let res2 = EvaluationEngine::evaluate(&storage, run_id, &construct_id).await?;
        assert_eq!(res2.status, ConstructStatus::Certified);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_certify_mintclip() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("test.sqlite");
        let artifact_root = dir.path().join("artifacts");
        let storage = Storage::new(db_path.to_str().unwrap(), artifact_root.to_str().unwrap()).await?;
        
        let run_id = "mintclip_run";
        storage.event_log.create_run_row(run_id, "mintclip.yaml", "workspace", None).await?;
        
        // Define MintClip Spec
        let spec = ConstructSpec {
            id: "mintclip_construct_v1".into(),
            kind: ConstructKind::Cli,
            name: "MintClip Clipboard Manager".into(),
            description: "A Linux Mint clipboard manager with persistent history and CLI interface".into(),
            root: ".".into(),
            interface: Interface { exports: vec![] },
            invariants: vec![],
            acceptance: Acceptance { 
                required_probes: vec![
                    "probe:unit_tests".into(),
                    "probe:cli_integration".into(),
                    "probe:lint".into(),
                    "probe:format".into()
                ], 
                pass_policy: PassPolicy { mode: PassPolicyMode::All } 
            },
            dependencies: None,
            metadata: None,
        };
        
        let construct_id = storage.constructs.create(run_id, spec.kind.clone(), &spec.name, &spec.root).await?;
        storage.constructs.add_spec(&construct_id, &spec).await?;
        
        // 1. Implementation Applied (Commit Boundary)
        storage.event_log.append_event(run_id, "PatchApplied", serde_json::json!({
            "touchedPaths": ["src/mintclip/clipboard.py", "src/mintclip/cli.py", "tests/test_clipboard.py", "tests/test_cli_integration.py"]
        })).await?;
        
        // 2. Probes pass AFTER mutation
        for probe_id in &["unit_tests", "cli_integration", "lint", "format"] {
            storage.event_log.append_event(run_id, "ProbeFinished", serde_json::json!({
                "probeId": probe_id,
                "status": "pass"
            })).await?;
        }
        
        // 3. Evaluate
        let res = EvaluationEngine::evaluate(&storage, run_id, &construct_id).await?;
        println!("MintClip Certification Result: {:?}", res.status);
        assert_eq!(res.status, ConstructStatus::Certified);
        
        Ok(())
    }
}
