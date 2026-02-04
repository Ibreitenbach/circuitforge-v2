use std::{path::{Path, PathBuf}}; use anyhow::Context; use serde_json::json; use tokio::fs;

/// Service for scaffolding initial project structures and environment specs.
/// Uses tokio::fs for non-blocking I/O.
pub struct ScaffoldService;

impl ScaffoldService {
  pub fn new() -> Self { Self }

  /// Persists a problem request to a JSON specification file.
  pub async fn save_problem_spec(&self, req: &crate::ipc::commands::CompileProblemReq) -> anyhow::Result<String> {
    let out_dir = std::env::temp_dir().join("circuitforge_problem_specs");
    fs::create_dir_all(&out_dir).await?;

    let spec_id = uuid::Uuid::new_v4();
    let path = out_dir.join(format!("{}.json", spec_id));

    let problem_spec = json!({
      "id": spec_id.to_string(),
      "prompt": req.prompt,
      "languageProfile": req.language_profile,
      "difficulty": req.difficulty,
      "team": req.team,
      "generatedAt": chrono::Utc::now().to_rfc3339()
    });

    fs::write(&path, serde_json::to_vec_pretty(&problem_spec)?).await?;
    Ok(path.to_string_lossy().to_string())
  }

  /// Scaffolds a complete environment from a saved problem spec.
  pub async fn scaffold_environment(&self, problem_spec_path: &str, out_dir: &str) -> anyhow::Result<(String, String)> {
    fs::create_dir_all(out_dir).await?;

    let problem_spec_p = Path::new(problem_spec_path);
    anyhow::ensure!(problem_spec_p.exists(), "problem spec not found: {}", problem_spec_path);

    let content = fs::read_to_string(problem_spec_p).await
      .with_context(|| format!("reading problem spec: {}", problem_spec_path))?;
    let problem_spec: serde_json::Value = serde_json::from_str(&content)?;

    let language = problem_spec.get("languageProfile")
      .and_then(|v| v.as_str())
      .unwrap_or("python");

    // Better path resolution for templates
    let template_root = self.resolve_template_path(language);
    let repo_template_path = PathBuf::from(out_dir).join("repo_template");
    fs::create_dir_all(&repo_template_path).await?;

    if template_root.exists() {
      // Copy template files to repo_template
      let src = template_root.clone();
      let dst = repo_template_path.clone();
      tokio::task::spawn_blocking(move || {
          copy_dir_sync(&src, &dst)
      }).await??;
    } else {
      // Generate minimal starter files in repo_template
      self.generate_starter_files(&repo_template_path, language).await?;
    }

    // Always generate EnvironmentSpec.yaml from problem spec
    let env_spec_path = PathBuf::from(out_dir).join("EnvironmentSpec.yaml");
    self.generate_env_spec(&env_spec_path, language, &problem_spec).await?;

    Ok((env_spec_path.to_string_lossy().to_string(), repo_template_path.to_string_lossy().to_string()))
  }
  
  fn resolve_template_path(&self, language: &str) -> PathBuf {
      // Security: sanitize language to alphanumeric + underscore only (prevent path traversal)
      let safe_language: String = language.chars()
          .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
          .collect();
      let safe_language = if safe_language.is_empty() { "unknown".to_string() } else { safe_language };

      // 1. Check environment variable override
      if let Ok(base) = std::env::var("CF_TEMPLATES_ROOT") {
          return PathBuf::from(base).join(format!("{}_cli_basic", safe_language));
      }

      // 2. Fallback to exe relative path
      let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.to_path_buf())).unwrap_or_else(|| PathBuf::from("."));
      exe_dir.join("scenarios/templates").join(format!("{}_cli_basic", safe_language))
  }

  /// Generates starter files in the repo_template directory
  async fn generate_starter_files(&self, repo_template: &PathBuf, language: &str) -> anyhow::Result<()> {
    let starter_files: Vec<(&str, &str)> = match language {
      "python" => vec![
        ("main.py", "# CircuitForge starter\ndef main():\n    print('Hello')\n\n\nif __name__ == '__main__':\n    main()\n"),
        ("test_main.py", "import main\n\n\ndef test_placeholder():\n    assert True\n"),
        ("requirements-dev.txt", "pytest>=7.0.0\nflake8>=6.0.0\n")
      ],
      "rust" => vec![
        ("Cargo.toml", "[package]\nname = \"starter\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[dependencies]\n"),
        ("src/main.rs", "fn main() { println!(\"Hello\"); }\n#[cfg(test)]\nmod tests {\n    #[test] fn it_works() { assert_eq!(2 + 2, 4); }\n}\n")
      ],
      "javascript" => vec![
        ("package.json", "{\n  \"name\": \"starter\",\n  \"version\": \"1.0.0\",\n  \"scripts\": { \"test\": \"jest\", \"lint\": \"eslint .\" },\n  \"devDependencies\": { \"jest\": \"^29.0.0\", \"eslint\": \"^8.0.0\" }\n}"),
        ("index.js", "module.exports = { hello: () => 'Hello' };\n"),
        ("index.test.js", "const { hello } = require('./index');\ntest('hello returns Hello', () => { expect(hello()).toBe('Hello'); });\n")
      ],
      "typescript" => vec![
        ("package.json", "{\n  \"name\": \"starter\",\n  \"version\": \"1.0.0\",\n  \"scripts\": { \"build\": \"tsc\", \"test\": \"jest\", \"lint\": \"eslint . --ext .ts\" },\n  \"devDependencies\": { \"typescript\": \"^5.0.0\", \"jest\": \"^29.0.0\", \"ts-jest\": \"^29.0.0\", \"@types/jest\": \"^29.0.0\", \"eslint\": \"^8.0.0\", \"@typescript-eslint/parser\": \"^6.0.0\", \"@typescript-eslint/eslint-plugin\": \"^6.0.0\" }\n}\n"),
        ("tsconfig.json", "{\n  \"compilerOptions\": { \"target\": \"ES2020\", \"module\": \"commonjs\", \"strict\": true, \"esModuleInterop\": true, \"skipLibCheck\": true, \"outDir\": \"./dist\" },\n  \"include\": [\"src/**/*\"],\n  \"exclude\": [\"node_modules\"]\n}\n"),
        ("src/index.ts", "export const hello = (): string => 'Hello';\n"),
        ("src/index.test.ts", "import { hello } from './index';\ntest('hello returns Hello', () => { expect(hello()).toBe('Hello'); });\n"),
        ("jest.config.js", "module.exports = { preset: 'ts-jest', testEnvironment: 'node' };\n")
      ],
      _ => vec![("README.md", "# CF Project")]
    };

    for (name, content) in starter_files {
      let path = repo_template.join(name);
      if let Some(parent) = path.parent() { fs::create_dir_all(parent).await?; }
      fs::write(path, content).await?;
    }
    Ok(())
  }

  /// Generates EnvironmentSpec.yaml from problem spec
  async fn generate_env_spec(&self, env_spec_path: &PathBuf, language: &str, problem_spec: &serde_json::Value) -> anyhow::Result<()> {
    let test_cmd: Vec<String> = match language {
      "python" => vec!["sh".to_string(), "-c".to_string(), "pip install -r requirements-dev.txt -q && python3 -m pytest -v".to_string()],
      "rust" => vec!["cargo".to_string(), "test".to_string()],
      "javascript" | "typescript" => vec!["sh".to_string(), "-c".to_string(), "npm install && npm test".to_string()],
      _ => vec!["echo".to_string(), "No runner".to_string()],
    };

    // Quality check commands - distinct from unit tests
    let quality_cmd: Vec<String> = match language {
      "python" => vec!["sh".to_string(), "-c".to_string(), "pip install -r requirements-dev.txt -q && python3 -m flake8 .".to_string()],
      "rust" => vec!["cargo".to_string(), "clippy".to_string(), "--".to_string(), "-D".to_string(), "warnings".to_string()],
      "javascript" | "typescript" => vec!["sh".to_string(), "-c".to_string(), "npm install && npm run lint".to_string()],
      _ => vec!["echo".to_string(), "No quality check".to_string()],
    };

    let env_id = uuid::Uuid::new_v4().to_string();
    let prompt = problem_spec.get("prompt").and_then(|v| v.as_str()).unwrap_or("CircuitForge Project");
    let difficulty = problem_spec.get("difficulty").and_then(|v| v.as_str()).unwrap_or("easy");
    
    // Difficulty-based node generation
    let mut nodes = vec![
        json!({ "id": "start", "kind": "chip", "label": "Start", "pos": { "x": 0, "y": 0 } })
    ];
    let mut edges = vec![];
    
    let (gate_node_id, gates) = match difficulty {
        "hard" => {
            // Hard mode: two probes (quality + unit) both must pass via All predicate
            nodes.push(json!({ "id": "probe_q", "kind": "probe", "label": "Quality", "pos": { "x": 200, "y": -100 }, "validatorId": "v_quality" }));
            nodes.push(json!({ "id": "probe_u", "kind": "probe", "label": "Unit", "pos": { "x": 200, "y": 100 }, "validatorId": "v_unit" }));
            nodes.push(json!({ "id": "gate_u", "kind": "fuse", "label": "Gate", "pos": { "x": 400, "y": 0 }, "gateId": "g_hard" }));
            edges.push(json!({ "id": "e1", "kind": "trace", "from": { "nodeId": "start" }, "to": { "nodeId": "probe_q" }, "directed": true }));
            edges.push(json!({ "id": "e2", "kind": "trace", "from": { "nodeId": "start" }, "to": { "nodeId": "probe_u" }, "directed": true }));
            edges.push(json!({ "id": "e3", "kind": "trace", "from": { "nodeId": "probe_q" }, "to": { "nodeId": "gate_u" }, "directed": true }));
            edges.push(json!({ "id": "e4", "kind": "trace", "from": { "nodeId": "probe_u" }, "to": { "nodeId": "gate_u" }, "directed": true }));
            ("gate_u", vec![
                json!({ "id": "g_hard", "kind": "fuse", "predicate": { "type": "all", "of": [
                    { "type": "probePass", "probeId": "probe_q" },
                    { "type": "probePass", "probeId": "probe_u" }
                ]}})
            ])
        }
        _ => {
            // Easy/default mode: single probe
            nodes.push(json!({ "id": "probe_unit", "kind": "probe", "label": "Unit Tests", "pos": { "x": 200, "y": 0 }, "validatorId": "v_unit" }));
            nodes.push(json!({ "id": "gate_unit", "kind": "fuse", "label": "Unit Gate", "pos": { "x": 400, "y": 0 }, "gateId": "g_unit" }));
            edges.push(json!({ "id": "e1", "kind": "trace", "from": { "nodeId": "start" }, "to": { "nodeId": "probe_unit" }, "directed": true }));
            edges.push(json!({ "id": "e2", "kind": "trace", "from": { "nodeId": "probe_unit" }, "to": { "nodeId": "gate_unit" }, "directed": true }));
            ("gate_unit", vec![
                json!({ "id": "g_unit", "kind": "fuse", "predicate": { "type": "probePass", "probeId": "probe_unit" } })
            ])
        }
    };

    nodes.push(json!({ "id": "finish", "kind": "terminal", "label": "Finish", "pos": { "x": 600, "y": 0 } }));
    edges.push(json!({ "id": "ef", "kind": "trace", "from": { "nodeId": gate_node_id }, "to": { "nodeId": "finish" }, "directed": true }));

    // Build validators list based on difficulty
    let validators = if difficulty == "hard" {
        vec![
            json!({ "id": "v_unit", "kind": "test", "cmd": test_cmd }),
            json!({ "id": "v_quality", "kind": "quality", "cmd": quality_cmd })
        ]
    } else {
        vec![json!({ "id": "v_unit", "kind": "test", "cmd": test_cmd })]
    };

    let env_spec_obj = json!({
      "id": env_id,
      "name": format!("CF: {}", prompt),
      "languageProfile": language,
      "team": problem_spec.get("team").cloned().unwrap_or(json!({
          "sparks": [
              {"id": "ac1", "kind": "ac"},
              {"id": "dc1", "kind": "dc"}
          ],
          "dcConfigMode": "tier1"
      })),
      "board": { "nodes": nodes, "edges": edges },
      "gates": gates,
      "validators": validators,
      "permissions": {
        "ac": { "allowEditPaths": ["**/*.py", "**/*.js", "**/*.ts", "**/*.rs"] },
        "dc": { "allowConfigPaths": ["config.yaml", "settings.json"] }
      },
      "hp": {
        "max": 100,
        "start": 100,
        "damage": { "unitFail": 5, "integrationFail": 10, "qualityFail": 3, "policyViolationAttempt": 15 },
        "heal": { "unlockGateUnit": 2, "unlockGateIntegration": 5, "cleanRunAll": 10 }
      }
    });

    let env_spec_yaml = serde_yaml::to_string(&env_spec_obj)?;
    fs::write(env_spec_path, env_spec_yaml).await?;
    Ok(())
  }
}

fn copy_dir_sync(src: &PathBuf, dst: &PathBuf) -> anyhow::Result<()> {
  use walkdir::WalkDir;
  use std::fs;
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