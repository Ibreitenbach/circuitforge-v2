# CircuitForge – Foundational Technical Design

**Subtitle:** Building Real Software from First Principles using Sparks, Circuits, and Cooperative Validation

**Status:** Concept locked → Execution discipline phase

---

## 0. Design Intent (Read This First)

CircuitForge exists to **turn software development into a system of composable first‑principle primitives** that can be assembled—like LEGO bricks—into real, production‑grade software systems.

The system deliberately rejects:
- performance theater
- social validation as truth
- opaque agent reasoning
- unbounded iteration

In its place, CircuitForge enforces **cooperative validation under cost**, where progress only occurs when claims are proven with evidence.

The spark‑and‑circuit metaphor is not cosmetic. It is a **semantic mapping** of software reality into enforceable mechanics.

---

## 1. Core Thesis

> **Software can be constructed from a small set of irreducible primitives, if validation is structural and cooperation is enforced.**

CircuitForge operationalizes this thesis by:
- encoding development actions as *claims*
- forcing claims through *validators*
- grounding truth in *evidence*
- limiting iteration through *cost*
- preserving all state via *events*

The result is not a game, but a **construction substrate**.

---

## 2. Non‑Goals (Hard Constraints)

CircuitForge explicitly does **not** attempt to:
- replace programming languages
- hide complexity behind prompts
- optimize for vibes or aesthetics
- act as an autonomous AGI

It **does** aim to:
- build correct software
- make progress measurable
- keep failure explicit
- support humans and LLMs as equal actors

---

## 3. First‑Principle Primitives (Frozen Vocabulary)

All future features **must reduce** to these primitives.

### 3.1 Event
**Definition:** An immutable record of something that occurred.

- Append‑only
- Time‑ordered per run
- The sole source of truth

> There is no hidden state.

---

### 3.2 Action (Spark)
**Definition:** An intentional attempt to change the system.

Examples:
- apply a patch
- run a probe
- unlock a gate

Every action implicitly asserts a **claim**.

---

### 3.3 Claim
**Definition:** A statement about reality implied by an action.

Examples:
- “This code fixes the bug”
- “This refactor preserves behavior”
- “The model meets accuracy threshold”

Claims are not trusted.

---

### 3.4 Probe (Validator)
**Definition:** A deterministic process that evaluates a claim.

- tests
- linters
- benchmarks
- formal checks

Probes have no authority beyond what they can prove.

---

### 3.5 Evidence (Artifact)
**Definition:** Immutable output produced by a probe.

- stdout / stderr
- diffs
- metrics
- reports

> Evidence is the only accepted currency of truth.

---

### 3.6 Capsule
**Definition:** A composable unit of intent.

Capsules are:
- declarative
- parameterized
- replayable
- embeddable

Examples:
- patch capsule
- probe capsule
- scenario capsule

Capsules are the **LEGO bricks** of the system.

---

### 3.7 Construct (New, Critical)
**Definition:** A real software artifact assembled from capsules.

Examples:
- CLI tool
- HTTP service
- library
- ML pipeline

A construct declares:
- inputs
- invariants
- acceptance probes
- extension points

> CircuitForge exists to produce Constructs.

---

## 4. The Spark & Circuit Metaphor (Semantics, Not Art)

### 4.1 Sparks

- Represent *intent*
- Initiate actions
- Are consumed

Unbounded sparks do not exist.

---

### 4.2 Circuits

- Represent *structure*
- Encode dependencies
- Enforce order

A closed circuit means:
> All invariants satisfied.

---

### 4.3 Gates, Fuses, Relays

- **Gates:** Preconditions
- **Fuses:** Failure limits
- **Relays:** Conditional propagation

These are execution controls, not flavor.

---

## 5. Cooperative Validation Model

CircuitForge enforces cooperation structurally:

- Builders (AC) propose changes
- Validators (DC) challenge claims
- Progress requires mutual satisfaction

No actor can advance alone.

This eliminates:
- persuasion without proof
- dominance by verbosity
- silent regressions

---

## 6. Cost Model (Why This Converges)

All meaningful actions incur cost:

- limited sparks
- HP loss
- fuse burn
- turn limits

> Cost forces prioritization.

Without cost, agents loop.
With cost, agents optimize.

---

## 7. Memory, Truth, and Authority

Agents may have persistent memory.

CircuitForge does **not** trust memory.

**Authority order:**
1. Event log
2. Filesystem state
3. Probe evidence
4. Agent memory

Memory may guide decisions but never override evidence.

---

## 8. Determinism & Replay

Given:
- same inputs
- same events
- same probes

CircuitForge must produce:
- identical outcomes

This is mandatory for:
- audits
- benchmarking
- learning from failure

---

## 9. From Arena to Factory

CircuitForge progresses through three phases:

### Phase 1 – Arena
- agents patch files
- probes enforce truth

### Phase 2 – Construction
- constructs defined
- invariants enforced incrementally

### Phase 3 – Factory
- construct graphs
- reusable pipelines
- reproducible outputs

This document locks the transition from Phase 1 → Phase 2.

---

## 10. Execution Discipline Rules

These rules override convenience:

1. If it cannot be validated, it does not exist
2. If it cannot be replayed, it is a bug
3. If it cannot be composed, it is a special case
4. If it cannot be audited, it is untrusted

---

## 11. Success Criteria

CircuitForge is succeeding when:

- real software is produced end‑to‑end
- agents converge faster over time
- failures teach, not obscure
- humans trust the outputs without trust in agents

---

## 12. Closing Statement

CircuitForge is not a shortcut.
It is a **discipline encoded as a system**.

The spark‑and‑circuit metaphor survives because it is *true*, not because it is cute.

From this point forward, all work is execution.

---

**End of Foundational Design Document**

