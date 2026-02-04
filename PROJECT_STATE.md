# CircuitForge: Project State 📜

**Status:** MVP Operational 🚀
**Audit Verdict:** PASS ✅ (10 cycles to Consilience)
**Date:** February 2, 2026

## 1. Core Architecture
CircuitForge is a multi-agent adversarial development environment built on Tauri (Rust) and React. It uses a "Spark" system where agents collaborate or compete to solve technical puzzles.

- **Persistence Layer:** SQLite-backed event sourcing. Every action (moves, edits, probes) is durable and survives server restarts.
- **Security System:** Role-based access control (ACLs) enforced at the World level. Policies are logged as events, and violations result in HP damage.
- **Scaffolding Service:** Automated environment generation from JSON specifications (`EnvironmentSpec`). Supports Python, Rust, and TypeScript.

## 2. Recent Major Fixes
- **HP Replay Bug:** Resolved issue where spark health was lost on restart. `apply_event` now correctly reconstructs HP from historical deltas.
- **Non-Blocking I/O:** Migrated scaffolding service to `tokio::fs` and `spawn_blocking` to prevent UI thread freezes.
- **Security Audit Durability:** Fixed "Lazy Error Handling" in IPC commands. Side effects like `PolicyViolation` logging are now strictly awaited.
- **Hard Mode Graph Logic:** Fixed disconnected finish nodes and broken gate predicates in complex topologies.
- **Environment Bootstrapping:** Scaffolds now automatically include `npm install` or `pip install` in validation commands for "out of the box" functionality.

## 3. Current Feature Set
| Feature | Status | Description |
| :--- | :--- | :--- |
| **Run Management** | ✅ | Create, Load, and Reset simulation runs. |
| **Spark Engine** | ✅ | Movement, Observations, and Energize actions. |
| **Adversarial Audit** | ✅ | The Octopus Protocol: harsh model-driven review. |
| **Validation Probes**| ✅ | Integrated Pytest, Cargo Test, and Jest support. |
| **Staging/Patches** | ✅ | AC sparks edit files; DC sparks apply patches. |
| **HP/Game Loops** | ✅ | Dynamic damage/healing based on performance. |

## 4. Known Issues & Technical Debt
- **Rust Warnings:** Approximately 10 warnings (unused variables/mutability) remain in the `src-tauri` build.
- **UI Status:** Frontend is functional but minimal. Subscriptions to `board.diff` are active but visual rendering is basic.
- **Template Resolution:** Path resolution for templates relies on `std::env::current_exe()`, which may be brittle in packaged AppImages or MSI installers.

## 5. Roadmap
1.  **UI Polishing:** Implement a rich 2D/3D board view for real-time spark tracking.
2.  **Advanced Validators:** Add `Ruff`, `Mypy`, and `CodeQL` as first-class validator types.
3.  **Collaborative Handoffs:** Refine the patch-sharing UI for more intuitive AC/DC rendezvous.
4.  **Scenario Editor:** A web-based tool to generate `EnvironmentSpec.yaml` files visually.
