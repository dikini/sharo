# MVP Slice 005 Verification And Hardening Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: close MVP verification matrix with executable evidence and harden restart and invariants.
Architecture: convert every remaining matrix row to runnable tests and evidence-producing checks. Keep runtime behavior stable while increasing confidence and observability artifacts.
Tech Stack: Rust tests, shell policy checks, matrix-to-test mapping docs.
Template-Profile: tdd-strict-v1

---

### Task 1: Map Verification Matrix Rows To Tests

**Files:**
- Create: `docs/plans/2026-03-05-mvp-verification-matrix-map.md`
- Test: `scripts/tests/test-mvp-matrix-map.bats`

**Preconditions**
- Slices 001-004 are implemented.

**Invariants**
- Every matrix row has one owning test id or documented not-implemented reason.

**Postconditions**
- Matrix mapping document is complete and machine-checkable.

**Tests (must exist before implementation)**

Unit:
- `matrix_map_has_unique_row_keys`

Property:
- `matrix_rows_have_test_binding`

Integration:
- `matrix_map_references_existing_tests`

**Red Phase (required before code changes)**

Command: `scripts/run-shell-tests.sh --all`
Expected: fails before map checker is added.

**Implementation Steps**

1. Add row-to-test mapping doc.
2. Add shell check to validate mapping references.

**Green Phase (required)**

Command: `scripts/run-shell-tests.sh --all`
Expected: shell tests pass with matrix map check.

### Task 2: Add Missing Recovery And Invariant Tests

**Files:**
- Modify: `crates/sharo-daemon/tests/*`
- Modify: `crates/sharo-core/tests/*`

**Preconditions**
- Matrix map indicates remaining gaps.

**Invariants**
- Recovery and trace continuity behavior remains explicit.

**Postconditions**
- Missing matrix rows now have passing automated tests.

**Tests (must exist before implementation)**

Unit:
- `step_terminal_state_is_explicit`

Property:
- `trace_continuity_preserved_on_restart`

Integration:
- `recovery_preserves_pending_approval_and_conflict_visibility`

**Red Phase (required before code changes)**

Command: `cargo test --workspace`
Expected: failing tests for newly added matrix-gap cases.

**Implementation Steps**

1. Add failing tests for uncovered matrix rows.
2. Implement minimal runtime changes to satisfy each row.

**Green Phase (required)**

Command: `cargo test --workspace`
Expected: workspace tests pass with all matrix rows covered.

### Task 3: Final MVP Gate And Evidence Packaging

**Files:**
- Modify: `docs/specs/mvp.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/tasks/tasks.csv`

**Preconditions**
- All matrix rows are test-backed and passing.

**Invariants**
- MVP claims are evidence-backed only.

**Postconditions**
- MVP readiness is explicitly documented and task tracking is closed.

**Tests (must exist before implementation)**

Unit:
- `mvp_readiness_checklist_has_no_open_required_items`

Property:
- `task_registry_states_consistent_with_mvp_gate`

Integration:
- `full_policy_and_test_gate_passes_on_mvp_state`

**Red Phase (required before code changes)**

Command: `scripts/check-fast-feedback.sh --all`
Expected: may fail before final closure updates.

**Implementation Steps**

1. Update MVP spec verification references with concrete evidence paths.
2. Mark slice tasks done with summary notes and changelog evidence.

**Green Phase (required)**

Command: `scripts/check-fast-feedback.sh --all`
Expected: full gate passes with MVP closure docs.

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated
