# CircuitForge Project State

## Overview
CircuitForge is a gamified code review platform built as a Tauri app (Rust backend + React frontend). It turns collaborative software development into a cooperative board game where two players (AC and DC) work together to pass gates and complete circuits.

## Architecture

### Technology Stack
- **Backend**: Rust (Tauri 2.x)
- **Frontend**: React + Vite + TypeScript
- **Storage**: SQLite (event sourcing)
- **IPC**: Tauri command system

### Directory Structure
```
circuitforge_starter/
├── src-tauri/                 # Rust backend
│   └── src/
│       ├── core/
│       │   └── world.rs       # Game logic & state management
│       ├── schemas/
│       │   └── environment.rs # Type definitions
│       ├── storage/
│       │   ├── mod.rs         # SQLite connection
│       │   ├── event_log.rs   # Event sourcing
│       │   ├── artifacts.rs   # Patch/evidence storage
│       │   └── tool_runs.rs   # Probe execution logs
│       ├── sandbox/
│       │   └── runner.rs      # Test/validator execution
│       ├── generator/
│       │   └── scaffold.rs    # Scenario scaffolding
│       ├── ipc/
│       │   ├── commands.rs    # Tauri IPC handlers
│       │   └── events.rs      # Frontend event emission
│       └── main.rs            # Entry point
├── ui/                        # React frontend
│   └── src/
│       ├── App.tsx            # Main component
│       ├── main.tsx           # Entry point
│       └── ipc/commands.ts    # Backend API calls
├── scenarios/                 # Example EnvironmentSpecs
└── docs/                      # Documentation
```

## Core Components

### 1. WorldService (`world.rs`)
The central game engine managing:
- **Run State**: Active game sessions with board snapshots
- **Spark Movement**: Player positioning on the circuit board
- **Gate Evaluation**: Predicate checking for fuses/relays
- **HP Management**: Damage/heal calculations
- **ACL Enforcement**: Permission validation for AC/DC roles

Key methods:
- `init_run()` - Create new game session
- `ac_edit_files()` - AC file modifications
- `ac_create_patch()` - Package changes for handoff
- `dc_apply_patch()` - DC applies AC's changes
- `dc_run_probe()` - Execute validators (tests)
- `dc_close_relay()` - Final merge gate

### 2. Storage Layer
**Event Sourcing Pattern:**
- All state changes persisted as events
- `apply_event()` reconstructs state from log
- Survives crashes/restarts

Tables:
- `runs` - Game session metadata
- `event_log` - Append-only event stream
- `artifacts` - Patches and evidence
- `tool_runs` - Validator execution history

### 3. Sandbox Runner (`runner.rs`)
Executes validators in isolated environment:
- Runs pytest, cargo test, npm test, etc.
- Captures stdout/stderr
- Returns pass/fail status with evidence

### 4. Generator (`scaffold.rs`)
Creates starter projects from EnvironmentSpec:
- Language-specific templates (Python, Rust, JS/TS)
- Quality validators (ruff, clippy, eslint)
- Test scaffolding

## Key Data Structures

### EnvironmentSpec (YAML)
Defines a complete scenario:
```yaml
id: scenario_id
name: "Display Name"
languageProfile: python|rust|javascript|typescript
team:
  sparks: [{id, kind: ac|dc}]
board:
  nodes: [{id, kind, label, pos, gateId?, validatorId?}]
  edges: [{from, to, directed, inlineGateIds?}]
gates: [{id, kind, predicate}]
validators: [{id, kind, cmd}]
permissions: {ac: {allowEditPaths}, dc: {denyEditPaths}}
hp: {max, start, damage, heal, antiThrash}
```

### Node Kinds
- **Chip**: Work stations (implementation areas)
- **Fuse**: Single-condition gates
- **Relay**: Multi-condition merge gates
- **Probe**: Test execution points

### Predicates
- `probePass`: Validator must succeed
- `patchCapsulePresent`: AC patch required
- `evidenceSetPresent`: All validators passed
- `all`: Multiple conditions combined

## Current Status

### Completed Features
- [x] Run initialization from EnvironmentSpec
- [x] AC file editing with ACL enforcement
- [x] Patch creation and handoff
- [x] DC patch application
- [x] Probe execution (pytest, cargo, npm)
- [x] Gate predicate evaluation
- [x] Relay closing with all predicates
- [x] HP damage/heal system
- [x] Event persistence (crash recovery)
- [x] 3-phase persist-first architecture
- [x] Path traversal protection

### Tested Scenarios
1. **Smoke Test** (Python): Simple transformer function
   - Result: PASS, HP 100/100
2. **Hard Scenario** (Python): KVStore + TTL + Journal
   - Result: PASS, HP 85/85, Coverage 93%

### Known Limitations
- Frontend UI is minimal (skeleton only)
- No real-time multiplayer (turn-based via file/chat)
- Composite validators run sequentially

## Next Steps
1. Complete frontend board visualization
2. Add WebSocket for real-time updates
3. Implement more scenario templates
4. Add replay/spectator mode
