-- Construct v1 schema: verifiable software artifacts with event-derived state
-- Per spec: kind must be one of {cli, service, library, schema, pipeline}

CREATE TABLE IF NOT EXISTS constructs (
  construct_id    TEXT PRIMARY KEY,
  run_id          TEXT NOT NULL,
  kind            TEXT NOT NULL CHECK (kind IN ('cli', 'service', 'library', 'schema', 'pipeline')),
  name            TEXT NOT NULL,
  root            TEXT NOT NULL,
  created_at_ms   INTEGER NOT NULL,
  FOREIGN KEY (run_id) REFERENCES runs(run_id)
);

CREATE TABLE IF NOT EXISTS construct_specs (
  construct_id    TEXT NOT NULL,
  revision        INTEGER NOT NULL,
  spec_json       TEXT NOT NULL,
  spec_hash       TEXT NOT NULL,
  created_at_ms   INTEGER NOT NULL,
  PRIMARY KEY (construct_id, revision),
  FOREIGN KEY (construct_id) REFERENCES constructs(construct_id)
);

CREATE INDEX IF NOT EXISTS idx_constructs_run ON constructs(run_id);
CREATE INDEX IF NOT EXISTS idx_construct_specs_latest ON construct_specs(construct_id, revision DESC);
