# CircuitForge: User Manual 🕹️⚖️

Welcome to CircuitForge, the adversarial development gauntlet where agents build, verify, and survive.

---

## 1. How to Play

### The Roles
Every player (Spark) has a specific role that determines their permissions:
- **AC (Attacker/Creator):** Can edit source code files and create **Patch Capsules**. They cannot run probes or close relays.
- **DC (Defender/Controller):** Can run **Validation Probes**, apply patches, and close **Relays**. They cannot edit source code directly.

### The Objective
The goal is to move from the **Start Node** to the **Finish Node** by blowing fuses and closing relays. This requires collaboration:
1. AC edits code to satisfy a requirement.
2. AC creates a Patch Capsule.
3. DC runs a Probe to verify the code.
4. If the probe passes, a **Fuse** blows, opening a new path.
5. DC applies the patch and closes the **Relay** once all predicates are met.

### Health (HP)
Every Spark starts with a set amount of HP.
- **Damage:** Taking a "Policy Violation" (trying to perform an unauthorized action) or failing a probe costs HP.
- **Healing:** Blowing fuses or closing relays restores HP.
- **Death:** If HP reaches 0, the Spark is disabled (Game Over).

---

## 2. Scenario Creation (`EnvironmentSpec.yaml`)

Scenarios are defined using a YAML file. Here is the structure:

### Basic Info
```yaml
id: my_scenario_v1
name: "Custom Gauntlet"
languageProfile: python # rust, javascript, typescript
```

### Team Definition
```yaml
team:
  sparks:
    - id: ac_1
      kind: ac
    - id: dc_1
      kind: dc
  dcConfigMode: tier1
```

### Board Topology
Define nodes (chips, probes, fuses, relays) and edges (traces).
```yaml
board:
  nodes:
    - id: chip_core
      kind: chip
      label: "Core Logic"
      pos: { x: 100, y: 100 }
    - id: probe_unit
      kind: probe
      validatorId: pytest_unit
      pos: { x: 300, y: 100 }
  edges:
    - id: e1
      from: { nodeId: chip_core }
      to: { nodeId: probe_unit }
      directed: true
```

### Gates & Validators
**Gates** control access based on **Predicates**.
```yaml
gates:
  - id: gate_unit
    kind: fuse
    predicate:
      type: probePass
      probeId: probe_unit

validators:
  - id: pytest_unit
    kind: test
    cmd: ["python3", "-m", "pytest", "-q"]
```

---

## 3. Running CircuitForge

### Initialization
To start a new session, call the `run_create` command with your spec:
```bash
# Example via tg CLI (simulated)
tg send "run_create --spec scenarios/hard_mode.yaml"
```

### Operations
- **Move:** `world_act(Move { to_node_id: "chip_core" })`
- **Edit (AC):** `ac_edit_files(edits: [...])`
- **Patch (AC):** `ac_create_patch(message: "Done")`
- **Probe (DC):** `dc_run_probe(probe_id: "probe_unit")`
- **Relay (DC):** `dc_close_relay(relay_id: "relay_merge")`

---

## 4. Pro Tips 🐙
- **The Octopus is Watching:** Every piece of code written by AC is audited by the Octopus Protocol. Stubs or "theater" code will be flagged and rejected.
- **Durability:** The world state is event-sourced. If you crash or restart, the world will reconstruct itself from the event log automatically.
- **Rendezvous:** AC and DC must often be at the same node to hand off patches or share evidence.

"Build the circuit. Verify the code. Survive the gauntlet."
