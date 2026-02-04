# CircuitForge – Construct v1 Implementation & Vertical Slice Plan

**Purpose**  
Provide a *single, authoritative document* that organizes:
- the Construct v1 technical specification
- the smallest possible vertical slice
- a file‑by‑file implementation checklist
- explicit alignment with CircuitForge’s existing storage + event‑log architecture

This document is intended to be executable, not aspirational.

---

## 0. Scope Lock

This document covers **Construct v1 only**.

Out of scope:
- ML constructs
- multi‑repo builds
- distributed execution
- automatic invariant inference

If it does not serve the goal of building **one real CLI tool end‑to‑end**, it does not belong here.

---

## 1. Construct v1 (Condensed Canonical Definition)

A **Construct** is a verifiable software artifact defined by:

1. **Interface** – what it exposes
2. **Invariants** – what must always hold
3. **Acceptance Probes** – how correctness is proven
4. **Root** – the workspace subtree it owns

A Construct is **certified** when all required probes pass *after* the most recent mutation under its root.

All Construct state is **event‑derived**.

---

## 2. Smallest Possible Vertical Slice (Factory Ignition)

This is the *minimum proof* that CircuitForge builds real software.

### Vertical Slice Checklist

1. Implement Construct tables + events
2. Implement `construct.evaluate()` as a pure fold
3. Convert one existing scenario into one Construct spec (CLI)
4. Make Claude + Gemini drive the Construct to `Certified`

No human patching.
No special cases.

---

## 3. Database & Storage Layer

### 3.1 SQL Migration

**File:** `src-tauri/src/storage/migrations/0002_constructs.sql`

Create:
- `constructs`
- `construct_specs`

Constraints:
- `construct_id` primary key
- `(construct_id, revision)` primary key
- foreign key to `runs(run_id)`

Indexes:
- `constructs(run_id)`
- `construct_specs(construct_id, revision DESC)`

Certification is **not** stored; it is derived.

---

### 3.2 Storage Module

**File:** `src-tauri/src/storage/constructs.rs`

Responsibilities:
- create construct
- insert spec revision
- fetch latest spec
- list constructs for run

Wire into:
- `src-tauri/src/storage/mod.rs`

---

## 4. Schema Types (Rust)

**File:** `src-tauri/src/schemas/construct.rs`

Defines:
- `ConstructSpec`
- `Invariant`
- `InterfaceExport`
- `Acceptance`
- `Dependency`
- `ConstructStatus`
- `EvaluateResult`

This file is the Rust mirror of the Construct v1 schema.

---

## 5. Core Evaluation Engine

**File:** `src-tauri/src/core/constructs.rs`

### `construct.evaluate(construct_id)`

A pure fold over:
- latest `ConstructSpec`
- event log
- probe results
- patch artifacts

Algorithm:
1. Identify last mutation under `construct.root`
2. For each required probe:
   - find latest probe result after mutation
3. Compute:
   - missing probes
   - failing probes
4. Return:
   - `Draft | Active | Certified | Regressed`

No UI state.
No agent memory.

---

## 6. Event Log Integration

New event types (append‑only):

- `ConstructDefined`
- `ConstructSpecUpdated`
- `ConstructProbeResult` (optional alias of existing probe finish event)

Certification events may be emitted but **must be recomputable**.

---

## 7. IPC Surface (Tauri)

**File:** `src-tauri/src/ipc/commands.rs`

Commands:
- `construct_define(spec)`
- `construct_update_spec(construct_id, spec)`
- `construct_get(construct_id)`
- `construct_evaluate(construct_id)`
- `construct_list(run_id)`

All commands:
- schema‑validated
- emit events *after commit*

---

## 8. UI Integration (Minimal)

### 8.1 IPC Wrappers

**File:** `ui/src/ipc/commands.ts`

Add strongly‑typed wrappers for all Construct IPC commands.

---

### 8.2 UI State

**File:** `ui/src/state/constructStore.ts`

Tracks:
- construct list
- selected construct
- latest evaluation result

---

### 8.3 UI Panel

**File:** `ui/src/App.tsx`

Add a minimal Constructs panel:
- list constructs
- view status
- evaluate construct
- run required probes

UI may not mark anything as done.

---

## 9. Scenario → Construct Conversion (CLI Reference)

**Scenario:** `python_cli_basic`

### 9.1 Construct Spec

**File:** `scenarios/templates/python_cli_basic/ConstructSpec.yaml`

Defines:
- `kind: cli`
- `root: .`
- interface exports
- must‑invariants
- acceptance probes (existing probe IDs)

---

### 9.2 Scenario Loader

Option A (minimal):
- UI loads ConstructSpec.yaml manually

Option B (preferred):
- scaffold copies ConstructSpec.yaml into workspace

---

## 10. Discipline Gates (Enforced Rules)

1. Certification only via `construct.evaluate()`
2. Any patch under `root` invalidates certification
3. Required probes must run *after* mutation
4. No silent mutation outside `root`

---

## 11. Success Criteria

Construct v1 is complete when:

- Claude + Gemini build the CLI construct to `Certified`
- A replayed run yields the same result
- A third party can verify correctness from artifacts

---

## 12. Execution Order (Recommended)

1. SQL migration
2. Storage module
3. Construct schema
4. Evaluation fold
5. IPC commands
6. UI wiring
7. CLI ConstructSpec
8. Agent‑driven run

No reordering.

---

## 13. Final Constraint

If a future change violates:
- event‑derived truth
- probe‑backed invariants
- composability

…it is rejected.

---

**End of Construct v1 Implementation & Vertical Slice Plan**

