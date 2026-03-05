# MVP Slice 001 Scenario A Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: implement the mandatory Scenario A success path with durable trace and artifact retrieval.
Architecture: build the minimal kernel and persistence path for read-oriented execution first, with explicit route decision and verification artifacts. Expand protocol and CLI only as required to execute and inspect Scenario A.
Tech Stack: Rust 2024, tokio, serde, local file-backed store.
Template-Profile: tdd-strict-v1

---

### Task 1: Define MVP Core Runtime Records For Scenario A

**Files:**
- Modify: `crates/sharo-core/src/protocol.rs`
- Create: `crates/sharo-core/src/runtime_types.rs`
- Test: `crates/sharo-core/tests/runtime_types_tests.rs`

**Preconditions**
- Existing IPC envelope tests pass.

**Invariants**
- Existing `Submit` and `Status` IPC behavior remains backward compatible during slice.

**Postconditions**
- Scenario A required task, step, trace, and artifact record shapes exist.

**Tests (must exist before implementation)**

Unit:
- `task_state_supports_scenario_a_transitions`

Property:
- `trace_event_sequence_is_monotonic`

Integration:
- `scenario_a_record_roundtrip_json`

**Red Phase (required before code changes)**

Command: `cargo test --package sharo-core runtime_types_tests -- --nocapture`
Expected: fails before new types are implemented.

**Implementation Steps**

1. Add core record structs and enums needed by Scenario A.
2. Keep IPC envelope structs stable while introducing new operation payloads.

**Green Phase (required)**

Command: `cargo test --package sharo-core runtime_types_tests`
Expected: new tests pass.

### Task 2: Add Durable Store And Scenario A Execution Path

**Files:**
- Create: `crates/sharo-daemon/src/store.rs`
- Modify: `crates/sharo-daemon/src/main.rs`
- Test: `crates/sharo-daemon/tests/scenario_a.rs`

**Preconditions**
- Scenario A types are available in core crate.

**Invariants**
- No hidden in-memory-only success path; exact records persist.

**Postconditions**
- Submit and execution produce retrievable task state, trace, and artifacts.

**Tests (must exist before implementation)**

Unit:
- `store_persists_task_trace_artifact_records`

Property:
- `persisted_records_are_id_stable`

Integration:
- `scenario_a_read_task_succeeds_with_verification_artifact`

**Red Phase (required before code changes)**

Command: `cargo test --package sharo-daemon scenario_a -- --nocapture`
Expected: fails before store and execution are implemented.

**Implementation Steps**

1. Add local durable store abstraction and file-backed implementation.
2. Implement minimal one-step read execution and verification artifact emission.

**Green Phase (required)**

Command: `cargo test --package sharo-daemon scenario_a`
Expected: scenario test passes.

### Task 3: Expose Scenario A Protocol And CLI Reads

**Files:**
- Modify: `crates/sharo-core/src/protocol.rs`
- Modify: `crates/sharo-cli/src/main.rs`
- Test: `crates/sharo-cli/tests/scenario_a_cli.rs`

**Preconditions**
- Daemon store and scenario execution are available.

**Invariants**
- CLI output remains deterministic and machine-parseable.

**Postconditions**
- CLI can open session, submit task, get task, get trace, and list artifacts for Scenario A.

**Tests (must exist before implementation)**

Unit:
- `cli_parses_scenario_a_commands`

Property:
- `scenario_a_cli_output_contains_stable_ids`

Integration:
- `cli_scenario_a_end_to_end`

**Red Phase (required before code changes)**

Command: `cargo test --package sharo-cli scenario_a_cli -- --nocapture`
Expected: fails before command and protocol expansion.

**Implementation Steps**

1. Add protocol operations needed by Scenario A only.
2. Add matching CLI subcommands and output summaries.

**Green Phase (required)**

Command: `cargo test --workspace`
Expected: workspace tests pass with Scenario A coverage.

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated
