# Rust Lint Hygiene Implementation Plan

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Property, and Integration tests.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: restore a passing workspace clippy gate with minimal, behavior-preserving fixes.
Architecture: fix the current concrete clippy failures first, then re-run the full workspace lint gate and tests to prove the baseline is restored. The plan follows `rust-skills` lint and test-hygiene guidance and avoids blanket `allow` attributes.
Tech Stack: Rust 2024, Clippy, workspace tests.
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-RUST-LINT-HYGIENE-PLAN-001, TASK-RUST-LINT-HYGIENE-SPEC-001

---

### Task 1: Remove current unit-struct and module-ordering lint failures

**Files:**

- Modify: `crates/sharo-core/tests/reasoning_connector_tests.rs`
- Modify: `crates/sharo-daemon/src/store.rs`

**Preconditions**

- Current clippy failures are reproducible.

**Invariants**

- Test intent and runtime behavior remain unchanged.
- No lint is suppressed with a blanket `allow`.

**Postconditions**

- The current `default_constructed_unit_structs` and `items_after_test_module` failures are removed.

**Tests (must exist before implementation)**

Unit:
- `clippy_default_constructed_unit_structs_regression_is_removed`

Property:
- `workspace_clippy_gate_remains_clean_after_fix_batch`

Integration:
- `cargo_clippy_all_targets_all_features_passes`

**Red Phase (required before code changes)**

Command: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: fails on the known test and module-ordering diagnostics.

**Implementation Steps**

1. Replace `UnitStruct::default()` with direct unit-struct construction where appropriate.
2. Move helper items in `store.rs` ahead of the `#[cfg(test)]` module or move the test module to the file end.
3. Keep the patch tightly scoped to the reported lint failures.

**Green Phase (required)**

Command: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: the current known lint failures are gone.

**Refactor Phase (optional but controlled)**

Allowed scope: the two files above only
Re-run: `cargo test --workspace`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated

### Task 2: Re-verify the full Rust policy baseline

**Files:**

- Modify: `CHANGELOG.md`

**Preconditions**

- Task 1 lint fixes are in place.

**Invariants**

- The repo policy commands remain the source of truth.
- No follow-up edit reintroduces clippy failures.

**Postconditions**

- The workspace passes both clippy and tests after the fix batch.

**Tests (must exist before implementation)**

Unit:
- `clippy_default_constructed_unit_structs_regression_is_removed`

Property:
- `workspace_clippy_gate_remains_clean_after_fix_batch`

Integration:
- `cargo_clippy_all_targets_all_features_passes`

**Red Phase (required before code changes)**

Command: `cargo test --workspace`
Expected: should already pass; any failure now indicates the lint cleanup changed behavior and must be corrected.

**Implementation Steps**

1. Run `cargo test --workspace`.
2. Run the full clippy gate again.
3. Update `CHANGELOG.md` with the lint-baseline restoration artifact once both commands pass.

**Green Phase (required)**

Command: `cargo test --workspace && cargo clippy --all-targets --all-features -- -D warnings`
Expected: both commands pass.

**Refactor Phase (optional but controlled)**

Allowed scope: none beyond changelog wording
Re-run: `scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated
