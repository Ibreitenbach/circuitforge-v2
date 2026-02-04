# CircuitForge Starter (Turnkey Skeleton)

This is a minimal starter tree for the CircuitForge MVP described in the Technical Design Document.

**What’s included**
- `src-tauri/` Rust core skeleton (IPC commands, storage, sandbox runner, generator stub, minimal world service)
- `ui/` frontend skeleton with IPC wrappers and event subscriptions (Vite/React wiring omitted on purpose; you can drop this into an existing Tauri + Vite setup)

**Important**
- Dependency versions in `src-tauri/Cargo.toml` are pinned for reproducibility but may need adjustment depending on your installed Tauri toolchain.
- This starter is focused on the vertical slice: generate env → create run → AC edit → patch → DC apply → DC run probe → evidence → gate open → relay close.

## Next
1. Create a new Tauri app (or integrate these directories into your existing repo).
2. Copy `src-tauri/` into your Tauri project root.
3. Wire `ui/` into your chosen frontend setup (Vite + React recommended).
