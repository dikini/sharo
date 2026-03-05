# MVP Slice 003 Scenario C Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: implement overlap detection and durable coordination visibility without full arbitration.
Architecture: add coordination records (`intent`, `claim`, `conflict`, `channel`) with simple detection and explicit retrieval linkage. Keep arbitration logic deferred.
Tech Stack: Rust 2024, daemon store, protocol read surfaces.
Template-Profile: tdd-strict-v1

---

### Task 1: Add Coordination Record Types And Persistence

**Files:**
- Create: `crates/sharo-core/src/coordination.rs`
- Modify: `crates/sharo-daemon/src/store.rs`
- Test: `crates/sharo-daemon/tests/coordination_store.rs`

**Preconditions**
- Scenario A and B persistence model is active.

**Invariants**
- Coordination records are exact records with durable identifiers.

**Postconditions**
- Intent, claim, conflict, and channel records can be persisted and reloaded.

**Tests (must exist before implementation)**

Unit:
- `coordination_record_schema_roundtrip`

Property:
- `coordination_record_ids_are_stable`

Integration:
- `coordination_records_survive_restart`

**Red Phase (required before code changes)**

Command: `cargo test --package sharo-daemon coordination_store -- --nocapture`
Expected: fails before coordination store support exists.

**Implementation Steps**

1. Add coordination type definitions.
2. Add storage and lookup functions for coordination records.

**Green Phase (required)**

Command: `cargo test --workspace`
Expected: tests pass with coordination persistence.

### Task 2: Detect Overlap And Emit Conflict Records

**Files:**
- Modify: `crates/sharo-daemon/src/main.rs`
- Test: `crates/sharo-daemon/tests/scenario_c_overlap.rs`

**Preconditions**
- Coordination persistence is implemented.

**Invariants**
- Overlap detection does not silently rewrite or auto-resolve tasks.

**Postconditions**
- Overlap condition emits and persists conflict records linked to tasks.

**Tests (must exist before implementation)**

Unit:
- `overlap_detector_flags_resource_scope_collision`

Property:
- `same_input_overlap_detection_is_repeatable`

Integration:
- `scenario_c_overlap_is_visible_without_arbitration`

**Red Phase (required before code changes)**

Command: `cargo test --package sharo-daemon scenario_c_overlap -- --nocapture`
Expected: fails before overlap detector and emission path.

**Implementation Steps**

1. Add simple overlap detector for declared scope collisions.
2. Persist intent, claim, conflict, and channel linkage.

**Green Phase (required)**

Command: `cargo test --workspace`
Expected: workspace tests pass.

### Task 3: Expose Coordination Summary In Normal Reads

**Files:**
- Modify: `crates/sharo-core/src/protocol.rs`
- Modify: `crates/sharo-cli/src/main.rs`
- Test: `crates/sharo-cli/tests/coordination_cli.rs`

**Preconditions**
- Conflict records are being generated.

**Invariants**
- No dedicated coordination API required for MVP.

**Postconditions**
- `task get`, `trace get`, or `artifacts list` include coordination summary fields when present.

**Tests (must exist before implementation)**

Unit:
- `protocol_includes_optional_coordination_summary`

Property:
- `coordination_summary_absent_when_no_overlap`

Integration:
- `cli_shows_coordination_summary_for_overlap`

**Red Phase (required before code changes)**

Command: `cargo test --package sharo-cli coordination_cli -- --nocapture`
Expected: fails before CLI visibility fields are added.

**Implementation Steps**

1. Add coordination summary fields to read responses.
2. Render summary in CLI read commands.

**Green Phase (required)**

Command: `cargo test --workspace`
Expected: workspace tests pass.

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated
