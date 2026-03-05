# Restart Trace Continuity Hardening Implementation Plan

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Property, and Integration tests.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: harden restart recovery by proving a succeeded Scenario A task keeps the same durable task and trace identity, ordered events, and visible output after daemon restart.
Architecture: add one restart-focused daemon integration scenario that records task/trace state before restart and compares it with the recovered state after restart. Keep runtime behavior unchanged unless the red test exposes a real persistence gap; any fix must preserve existing task/trace/artifact contracts.
Tech Stack: Rust 1.93+, `sharo-daemon` integration tests, existing store persistence and IPC retrieval paths.
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-RECOVERY-HARDENING-001

---

### Task 1: Prove Restart Preserves Scenario A Trace Continuity

**Files:**

- Modify: `crates/sharo-daemon/tests/scenario_a.rs`
- Modify: `crates/sharo-daemon/src/store.rs`
- Modify: `docs/tasks/tasks.csv`
- Modify: `CHANGELOG.md`
- Test: `crates/sharo-daemon/tests/scenario_a.rs`

**Preconditions**

- Scenario A already persists successful task, trace, and artifacts.
- Daemon restart recovery already exists for approval and conflict flows.

**Invariants**

- Restart must not allocate a new task or trace id for an already persisted task.
- Trace event ordering must remain monotonic after recovery.
- Recovered `task get` output must still expose the same succeeded state and `result_preview`.

**Postconditions**

- There is explicit automated evidence that Scenario A survives restart without task/trace drift.
- Any persistence gap revealed by the new test is fixed without weakening existing scenario behavior.

**Tests (must exist before implementation)**

Unit:
- `trace_event_sequence_is_monotonic`

Property:
- `trace_continuity_preserved_on_restart`

Integration:
- `scenario_a_success_survives_restart_with_same_trace_and_preview`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-daemon scenario_a_success_survives_restart_with_same_trace_and_preview -- --nocapture`
Expected: FAIL because the restart-specific Scenario A recovery test does not exist yet.

**Implementation Steps**

1. Add the failing restart scenario in `crates/sharo-daemon/tests/scenario_a.rs`.
2. If needed, make minimal persistence/recovery changes in `crates/sharo-daemon/src/store.rs` so recovered task/trace/preview data matches pre-restart state.
3. Update task registry and changelog once the recovery check is green.

**Green Phase (required)**

Command: `cargo test -p sharo-daemon scenario_a_success_survives_restart_with_same_trace_and_preview -- --nocapture`
Expected: PASS with recovered task and trace matching pre-restart state.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/sharo-daemon/tests/scenario_a.rs`, `crates/sharo-daemon/src/store.rs`
Re-run: `scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated
