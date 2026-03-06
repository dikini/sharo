# Task Result Preview Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: surface successful answer content in `task get` as a short preview while keeping `trace get` and `artifacts list` as the canonical provenance surfaces.
Architecture: extend the public task summary model with an optional `result_preview`, populate it only for succeeded tasks from the same successful model-output path that already feeds trace/artifact persistence, and print it in the CLI `task get` output. Keep failure, approval, and blocked paths unchanged so task metadata remains stable when no answer exists.
Tech Stack: Rust 1.93+, `sharo-core` protocol types, `sharo-daemon` store/runtime path, `sharo-cli` IPC output formatting.
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-RUNTIME-CONTENT-PREVIEW-001

---

### Task 1: Add Result Preview To Protocol And Persistence

**Files:**

- Modify: `crates/sharo-core/src/protocol.rs`
- Modify: `crates/sharo-daemon/src/store.rs`
- Test: `crates/sharo-daemon/tests/scenario_a.rs`

**Preconditions**

- Successful task submissions already persist `model_output` artifacts and `model_output_received` trace events.
- `task get` returns `TaskSummary` from persisted store state.

**Invariants**

- `trace get` and `artifacts list` remain the canonical answer/provenance surfaces.
- `result_preview` is absent for blocked, awaiting approval, denied, or failed tasks.
- Preview text is derived from the same model output content path as the successful artifact/trace records.

**Postconditions**

- Succeeded tasks persist a non-empty `result_preview`.
- Task retrieval exposes that preview through the protocol.
- Existing success/failure task states remain unchanged.

**Tests (must exist before implementation)**

Unit:
- `trace_and_artifact_envelopes_include_conformance_fields`

Property:
- `scenario_a_read_task_succeeds_with_verification_artifact`

Integration:
- `cli_scenario_a_end_to_end`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-daemon scenario_a_read_task_succeeds_with_verification_artifact -- --nocapture`
Expected: FAIL after asserting `result_preview` on succeeded task.

**Implementation Steps**

1. Extend `TaskSummary` with optional `result_preview`.
2. Populate `result_preview` for immediate-success and approval-success task paths in the daemon store.
3. Leave blocked and failure paths with `result_preview = None`.

**Green Phase (required)**

Command: `cargo test -p sharo-daemon scenario_a_read_task_succeeds_with_verification_artifact -- --nocapture`
Expected: PASS with preview visible on succeeded task.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/sharo-core/src/protocol.rs`, `crates/sharo-daemon/src/store.rs`, `crates/sharo-daemon/tests/scenario_a.rs`
Re-run: `cargo test -p sharo-daemon scenario_a_read_task_succeeds_with_verification_artifact -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated

### Task 2: Surface Result Preview In CLI Task Output

**Files:**

- Modify: `crates/sharo-cli/src/main.rs`
- Modify: `crates/sharo-cli/tests/scenario_a_cli.rs`
- Modify: `CHANGELOG.md`
- Modify: `docs/tasks/tasks.csv`

**Preconditions**

- `TaskSummary.result_preview` exists and is populated for succeeded tasks.
- CLI `task get` already prints task metadata from `GetTask`.

**Invariants**

- CLI continues to print `blocking_reason` and `coordination_summary`.
- CLI does not claim a preview exists when the protocol field is absent.
- Full output content remains available through `artifacts list`.

**Postconditions**

- `task get` prints `result_preview=<content|none>`.
- End-to-end CLI path shows answer content without forcing operators into trace/artifact commands.
- Existing CLI scenario coverage stays green.

**Tests (must exist before implementation)**

Unit:
- `trace_and_artifact_envelopes_include_conformance_fields`

Property:
- `scenario_a_read_task_succeeds_with_verification_artifact`

Integration:
- `cli_scenario_a_end_to_end`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-cli --test scenario_a_cli cli_scenario_a_end_to_end -- --nocapture`
Expected: FAIL after asserting `result_preview` in `task get` output.

**Implementation Steps**

1. Print `result_preview` in CLI `task get`, defaulting to `none`.
2. Update CLI integration tests to assert preview presence for succeeded tasks and absence semantics where relevant.
3. Update task registry and changelog for the operator-facing content surface improvement.

**Green Phase (required)**

Command: `cargo test -p sharo-cli --test scenario_a_cli cli_scenario_a_end_to_end -- --nocapture`
Expected: PASS with task output showing preview.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/sharo-cli/src/main.rs`, `crates/sharo-cli/tests/scenario_a_cli.rs`
Re-run: `scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated
