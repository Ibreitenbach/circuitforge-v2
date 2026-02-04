# CircuitForge – Construct v1 Specification

**Purpose:** Define the canonical, minimal, and enforceable representation of a *real software artifact* that CircuitForge can build via primitives (Actions, Claims, Probes, Evidence, Capsules, Events).

**Status:** v1 (locked spec)

---

## 1. Definition

A **Construct** is a *target artifact* (CLI, service, library, schema, pipeline) defined by:

1. **Interface** (what it exposes)
2. **Invariants** (what must always hold)
3. **Acceptance Probes** (how truth is proven)
4. **Composition Rules** (how it depends on other constructs/capsules)

> A Construct is **not** a prompt and not a backlog. It is a **verifiable contract**.

---

## 2. Design Principles (Non‑Negotiable)

1. **Evidence‑only completion:** A Construct is “done” only when acceptance probes pass.
2. **Incremental verifiability:** Progress is measured by satisfied invariants, not narrative.
3. **Composable closure:** Constructs must be buildable from capsules and other constructs.
4. **Deterministic replay:** Same inputs/events → same construct state.
5. **No special pleading:** If a requirement can’t be probed, it can’t be a v1 invariant.

---

## 3. Core Model

### 3.1 Identifiers

- `construct_id`: UUID
- `run_id`: UUID (CircuitForge run)
- `scenario_id`: string/uuid
- `revision`: monotonically increasing integer per construct (event‑derived)

### 3.2 Construct Kind

`kind ∈ { cli, service, library, schema, pipeline }`

v1 must support at least: `cli`, `service`, `library`.

---

## 4. Construct Schema (Canonical)

Below is the **authoritative v1 schema**. This should be represented as JSON Schema in code, but authored as YAML for humans.

```yaml
construct:
  id: "uuid"
  kind: "cli | service | library | schema | pipeline"
  name: "string"
  description: "string"

  # Where the construct lives in the workspace
  root: "relative/path"   # e.g., "apps/hello-cli" or "services/user-api"

  interface:
    # Declares the externally observable surface area.
    # Everything here must be probed.
    exports:
      - id: "string"          # stable key
        type: "command | http_endpoint | library_api | file_artifact"
        signature: "string"   # human-readable contract; not code
        notes: "string"

  invariants:
    # A set of statements that must hold. Each must map to a probe.
    - id: "string"            # stable key
      statement: "string"      # e.g., "cli returns 0 on success"
      severity: "must | should"
      probe_ref: "probe:<probe_id>"  # required for 'must'

  acceptance:
    # The minimum probe set that closes the circuit.
    required_probes:
      - "probe:<probe_id>"
    pass_policy:
      # all required probes must pass.
      mode: "all"

  dependencies:
    constructs:
      - construct_id: "uuid"
        version: "pinned | latest_in_run"
        mount: "relative/path"   # optional: where dependency is materialized
    capsules:
      - capsule_id: "uuid"
        role: "generator | patch | probe"

  build_plan:
    # Optional in v1; can be derived. Present if you want explicit steps.
    steps:
      - id: "string"
        action: "apply_patch | run_probe | generate | edit_file"
        params: {}
        gates:
          - "gate:<gate_id>"

  artifacts:
    # Populated by events; optional in authored spec.
    outputs:
      - export_id: "string"      # matches interface.exports[].id
        artifact_id: "hash/uuid"

  metadata:
    owner: "human | agent:<name>"
    tags: ["string"]
    created_at: "iso8601"
```

---

## 5. Construct Lifecycle (Event‑Derived)

A Construct exists and evolves only through events.

### 5.1 States

- **Draft** – defined but not yet proven
- **Active** – being built; some invariants satisfied
- **Certified** – all required probes pass (closed circuit)
- **Regressed** – previously certified but now failing probes
- **Archived** – frozen and no longer mutated

### 5.2 State Transition Rules

1. `Draft → Active` when first build action is applied
2. `Active → Certified` when all `acceptance.required_probes` pass in the same revision window
3. `Certified → Regressed` if any required probe fails after certification
4. `* → Archived` only via explicit action (requires gate)

> Certification is not a label. It is a computed property from evidence.

---

## 6. Probes and Evidence Binding

### 6.1 Probe Contract (v1)

A probe is valid for Construct v1 only if it produces:

- `status: pass|fail`
- deterministic command invocation (pinned toolchain or container image)
- at least one artifact:
  - `stdout/stderr` (always)
  - optionally structured output (JUnit/JSON)

### 6.2 Evidence Mapping

Each invariant **must** map to a probe. The binding is explicit:

- `invariants[].probe_ref` must exist for `severity: must`
- `acceptance.required_probes[]` is the certification set

---

## 7. Composition Semantics

### 7.1 Construct ↔ Construct

A Construct may depend on another Construct only via **observable outputs**:

- a package artifact
- a generated client
- a library path mounted into workspace

Dependency must be either:

- `pinned` (reproducible)
- `latest_in_run` (convenient but still event‑bound)

### 7.2 Capsule Roles

Capsules are attached to constructs with a role:

- `generator` – produces skeleton or boilerplate
- `patch` – incremental changes
- `probe` – validation

v1 rule: **Capsules cannot silently mutate outside the Construct root**.

---

## 8. Minimal Required Constructs (Reference Trio)

To prove the system can build real things, CircuitForge must ship three reference constructs.

### 8.1 CLI Construct (Required)

**Interface exports:**
- command: `hello-cli --name <str>`

**Must invariants:**
- exits 0 on valid input
- prints deterministic output format
- `--help` present
- tests pass

**Probes:**
- unit tests
- CLI golden test
- lint/format check

### 8.2 HTTP Service Construct (Required)

**Interface exports:**
- `GET /health`
- `POST /echo`

**Must invariants:**
- health returns 200 and JSON
- echo returns input payload
- tests pass

**Probes:**
- unit tests
- integration tests (spawn server, curl)

### 8.3 Library Construct (Required)

**Interface exports:**
- function signatures documented

**Must invariants:**
- semver‑compatible API surface for v1
- docs build
- tests pass

---

## 9. Database Representation (Suggested)

> The authored Construct spec can live as a file, but runtime truth is event‑sourced.

### 9.1 Tables

- `constructs(construct_id, run_id, kind, name, root, created_at)`
- `construct_specs(construct_id, revision, spec_json, spec_hash, created_at)`
- `construct_certifications(construct_id, revision, certified_at, evidence_event_id)` *(derived; optional materialization)*

### 9.2 Indexes

- `constructs(run_id)`
- `construct_specs(construct_id, revision desc)`

---

## 10. Event Types (v1)

These are the minimal additions to the event vocabulary:

- `ConstructDefined { construct_id, spec_hash }`
- `ConstructSpecUpdated { construct_id, spec_hash, diff_artifact_id }`
- `ConstructBuildStepApplied { construct_id, step_id, action_ref }`
- `ConstructProbeResult { construct_id, probe_id, status, artifact_ids[] }`
- `ConstructCertified { construct_id, revision, required_probes[] }` *(derived; may be stored for convenience)*
- `ConstructRegressed { construct_id, revision, failing_probes[] }` *(derived; may be stored)*

Rule: Derived events may be emitted, but certification truth must be recomputable from probe events.

---

## 11. IPC Surface (Agent / UI Contract)

Minimal commands needed to operate Construct v1:

- `construct.define(spec: ConstructSpec) -> {construct_id}`
- `construct.update_spec(construct_id, spec_patch) -> {revision}`
- `construct.get(construct_id) -> {spec, status, satisfied_invariants, last_probe_results}`
- `construct.run_probe(construct_id, probe_id) -> {probe_run_id}`
- `construct.evaluate(construct_id) -> {status: draft|active|certified|regressed, missing_probes[], failing_probes[]}`

All commands must be:
- schema‑validated
- idempotent where applicable
- emit events after commit

---

## 12. Discipline Gates (Anti‑Theater Enforcement)

To prevent narrative progress:

1. UI/agents may not mark anything “done.” Only `construct.evaluate()` can.
2. A patch capsule cannot claim success without immediately scheduling its acceptance probe.
3. A Construct cannot be exported unless certified (gateable override allowed).
4. Any change under Construct root invalidates certification until probes re‑pass.

---

## 13. Migration Plan (From MVP)

1. Add `constructs` + `construct_specs` tables
2. Add Construct events to event log
3. Implement `construct.evaluate()` fold
4. Convert existing scenarios into at least one Construct spec
5. Add one reference construct (CLI) end‑to‑end
6. Only then add service + library

---

## 14. “Closed Circuit” Definition

A Construct’s circuit is **closed** when:

- All `acceptance.required_probes` have most‑recent status = `pass`
- Those probe results were produced after the most recent mutation under `root`

Mutation detection can be:
- event‑based (preferred)
- or file hash snapshot artifact per revision

---

## 15. Out of Scope for v1 (Explicit)

- Multi‑repo constructs
- Distributed builds
- ML training constructs
- Automatic invariant inference

---

## 16. Success Metric for Construct v1

Construct v1 is complete when:

- Claude and Gemini can build the reference CLI construct to certification
- A third party can replay the run and independently verify certification
- A single exported bundle reproduces the certified artifact

---

**End of Construct v1 Specification**

