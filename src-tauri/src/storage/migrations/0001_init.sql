-- SQLx runs migrations in transactions automatically, no explicit BEGIN/COMMIT needed

CREATE TABLE IF NOT EXISTS runs (
  run_id           TEXT PRIMARY KEY,
  created_at_ms    INTEGER NOT NULL,
  env_spec_path    TEXT NOT NULL,
  workspace_path   TEXT NOT NULL,
  seed             INTEGER
);

CREATE TABLE IF NOT EXISTS events (
  run_id        TEXT NOT NULL,
  seq           INTEGER NOT NULL,
  ts_ms         INTEGER NOT NULL,
  type          TEXT NOT NULL,
  payload_json  TEXT NOT NULL,
  PRIMARY KEY (run_id, seq),
  FOREIGN KEY (run_id) REFERENCES runs(run_id)
);

CREATE INDEX IF NOT EXISTS idx_events_run_ts   ON events(run_id, ts_ms);
CREATE INDEX IF NOT EXISTS idx_events_run_type ON events(run_id, type);

CREATE TABLE IF NOT EXISTS run_seq (
  run_id     TEXT PRIMARY KEY,
  next_seq   INTEGER NOT NULL,
  FOREIGN KEY (run_id) REFERENCES runs(run_id)
);

CREATE TABLE IF NOT EXISTS artifacts (
  artifact_id      TEXT PRIMARY KEY,
  created_at_ms    INTEGER NOT NULL,
  byte_len         INTEGER NOT NULL,
  mime             TEXT,
  kind             TEXT,
  rel_path         TEXT NOT NULL,
  meta_json        TEXT
);

CREATE INDEX IF NOT EXISTS idx_artifacts_kind ON artifacts(kind);

CREATE TABLE IF NOT EXISTS run_artifacts (
  run_id         TEXT NOT NULL,
  artifact_id    TEXT NOT NULL,
  label          TEXT,
  created_at_ms  INTEGER NOT NULL,
  PRIMARY KEY (run_id, artifact_id, label),
  FOREIGN KEY (run_id) REFERENCES runs(run_id),
  FOREIGN KEY (artifact_id) REFERENCES artifacts(artifact_id)
);

CREATE INDEX IF NOT EXISTS idx_run_artifacts_run ON run_artifacts(run_id);

CREATE TABLE IF NOT EXISTS tool_runs (
  tool_run_id       TEXT PRIMARY KEY,
  run_id            TEXT NOT NULL,
  probe_id          TEXT,
  validator_id      TEXT,
  started_at_ms     INTEGER NOT NULL,
  finished_at_ms    INTEGER,
  status            TEXT,
  exit_code         INTEGER,
  cmd_json          TEXT NOT NULL,
  failure_signature TEXT,
  FOREIGN KEY (run_id) REFERENCES runs(run_id)
);

CREATE INDEX IF NOT EXISTS idx_tool_runs_run   ON tool_runs(run_id);
CREATE INDEX IF NOT EXISTS idx_tool_runs_probe ON tool_runs(run_id, probe_id);

CREATE TABLE IF NOT EXISTS tool_run_artifacts (
  tool_run_id  TEXT NOT NULL,
  artifact_id  TEXT NOT NULL,
  role         TEXT NOT NULL,
  PRIMARY KEY (tool_run_id, artifact_id, role),
  FOREIGN KEY (tool_run_id) REFERENCES tool_runs(tool_run_id),
  FOREIGN KEY (artifact_id) REFERENCES artifacts(artifact_id)
);
