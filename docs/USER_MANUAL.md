# CircuitForge User Manual

## What is CircuitForge?

CircuitForge is a cooperative coding game where two players work together to implement software features, pass tests, and complete a circuit board. Think of it as gamified pair programming with stakes.

## The Basics

### Players (Sparks)

**AC (Attacker/Creator)** - The Implementer
- Writes and edits source code
- Creates patches containing changes
- Cannot run tests or close gates
- Permissions: `src/**`, `docs/**`

**DC (Defender/Checker)** - The Validator
- Runs probes (tests, linters, type checkers)
- Applies patches from AC
- Closes gates when predicates are satisfied
- Cannot edit source code

### The Circuit Board

The game board is a directed graph representing your development workflow:

```
[Chip] ──→ [Fuse] ──→ [Chip] ──→ [Relay]
   │         🔒          │         🔒
   ↓                     ↓
[Probe]              [Probe]
```

**Node Types:**
- **Chip** - Work stations where implementation happens
- **Probe** - Test execution points (validators)
- **Fuse** - Single-condition gates (unlocked by one probe)
- **Relay** - Multi-condition merge gates (require patch + evidence)

### HP (Health Points)

Both sparks share an HP pool. Actions have consequences:

**Damage:**
- Unit test failure: -6 HP
- Integration failure: -16 HP
- Quality failure: -14 HP
- Policy violation: -90 HP

**Healing:**
- Unlock unit gate: +8 HP
- Unlock integration gate: +14 HP
- Clean run (all pass): +18 HP

**Anti-Thrash:** Repeating the same failure costs extra damage.

## How to Play

### Turn Structure

1. **AC Move**: Edit files, create patch
2. **DC Move**: Run probes, apply patches, close gates
3. Repeat until relay is closed or HP reaches 0

### AC Actions

**1. Edit Files**
```
ac_edit_files(run_id, spark_id, changes)
```
- Modify files in allowed paths (`src/**`)
- Changes tracked for patch creation

**2. Create Patch**
```
ac_create_patch(run_id, spark_id, message)
```
- Packages all recent edits
- Creates patch capsule for DC handoff
- Message describes the changes

### DC Actions

**1. Run Probe**
```
dc_run_probe(run_id, spark_id, probe_id)
```
- Executes the validator (pytest, cargo test, etc.)
- Creates evidence artifact on success
- Unlocks connected fuse if predicate satisfied

**2. Apply Patch**
```
dc_apply_patch(run_id, spark_id, patch_id)
```
- Applies AC's changes to workspace
- Required before probes test new code

**3. Close Relay**
```
dc_close_relay(run_id, spark_id, relay_id)
```
- Checks all predicates:
  - `patchCapsulePresent` - AC's patch received
  - `evidenceSetPresent` - All required probes passed
- Completes the circuit on success

## Gate Predicates

Gates unlock when their predicates are satisfied:

| Predicate | Requirement |
|-----------|-------------|
| `probePass` | Specific probe must pass |
| `patchCapsulePresent` | AC must have created a patch |
| `evidenceSetPresent` | Listed validators must all pass |
| `all` | All sub-predicates must be true |
| `coverageAtLeast` | Code coverage >= threshold |
| `lintPass` | Linter finds no errors |
| `typecheckPass` | Type checker passes |

## Example Game Flow

### Smoke Test Scenario

**Setup:** Python transformer function
- Gate: Unit tests must pass
- Relay: Needs patch + evidence

**Turn 1 - AC:**
```
1. Read failing test: transform("hello") == "HELLO"
2. Edit src/transformer.py:
   def transform(data):
       return data.upper()
3. Create patch: "Implement uppercase transformation"
```

**Turn 2 - DC:**
```
1. Run probe (pytest): PASS ✓
2. Gate unlocks! 🔓
3. Receive patch from AC
4. Close relay: SUCCESS ✓
```

**Result:** Circuit complete! HP intact.

### Hard Scenario (KVStore)

**Gates:**
1. Unit Fuse → `probe_unit` (basic get/set)
2. Durability Fuse → `probe_durability` (journal replay)
3. Integration Fuse → `probe_integration` (CLI works)
4. Quality Fuse → `probe_quality` (ruff + mypy + 90% coverage)

**Strategy:**
- AC implements incrementally
- DC validates each gate before proceeding
- Quality gate is composite - all 3 checks must pass
- Keep HP high by passing on first attempts

## Tips for Success

### For AC
- Read the tests first - understand what's expected
- Make small, focused changes
- Test locally before creating patch (if possible)
- Write clean, typed code for quality gates

### For DC
- Validate assumptions before running probes
- Apply patches before running tests on new code
- Check all predicates before attempting relay close
- Monitor HP - don't thrash on failures

### Team Strategy
- Communicate about what you're implementing
- AC: Describe changes in patch message
- DC: Report probe output clearly
- Coordinate on quality gate requirements

## Permissions Reference

### AC Allowed
- Edit: `src/**`, `docs/**`
- Create patches
- Move on board

### AC Denied
- Run probes
- Close gates
- Edit tests or configs

### DC Allowed
- Run probes
- Apply patches
- Close gates/relays
- Edit: `.ci/**`, `tooling/**`

### DC Denied
- Edit source code (`src/**`)
- Edit tests (`tests/**`)
- Create patches

## Troubleshooting

**"ACL Violation" error**
- You're trying to do something your role can't do
- AC can't run probes, DC can't edit source

**Gate won't unlock**
- Check predicate requirements
- Ensure probe actually passed (not just ran)
- Verify you're at the correct node

**Relay close fails**
- Need BOTH patch capsule AND evidence
- All required validators must have passed
- Check `evidenceSetPresent` validator list

**HP dropping fast**
- Anti-thrash is activated (same failure repeated)
- Focus on fixing root cause, not retrying
- Communicate with partner about blockers
