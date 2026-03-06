# Rust Lint Hygiene

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Define tests before implementation work.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Property, and Integration tests.
> - Run `scripts/doc-lint.sh` before commit.

Updated: 2026-03-06
Status: active
Owner: runtime
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-RUST-LINT-HYGIENE-SPEC-001, TASK-RUST-LINT-HYGIENE-PLAN-001

## Purpose

Restore a clean `cargo clippy --all-targets --all-features -- -D warnings` baseline so policy checks can trust the Rust lint gate.

## Scope

### In Scope

- Current clippy failures in test code and module organization.
- Guarding against recurrence with explicit verification coverage.
- Narrow cleanup only; no unrelated style churn.

### Out of Scope

- Enabling new lint groups beyond the existing gate.
- Large refactors whose only purpose is code-style normalization.
- Runtime behavior changes unrelated to lint failures.

## Core Terms

- `Lint Baseline`: a workspace state that passes the configured clippy invocation with warnings denied.
- `Test Hygiene`: test code written in forms that satisfy the active lint gate without suppressing diagnostics unnecessarily.
- `Module Layout Hygiene`: source-item ordering compatible with the lint configuration.

## Interfaces / Contracts

- The workspace clippy command must pass without `#[allow(...)]` unless a documented exception is strictly necessary.
- Unit structs should be constructed idiomatically.
- Test modules should remain at the end of the file or helper items should move ahead of them.

## Invariants

- Lint fixes must preserve current test intent and behavior.
- No production code path is weakened just to silence clippy.
- Verification commands in docs and policy scripts remain correct.

## Task Contracts

### Task 1: Remove Current Clippy Gate Failures

**Preconditions**

- Current workspace clippy invocation is reproducible.

**Invariants**

- Fixes are minimal and behavior-preserving.
- Test readability remains acceptable after lint cleanup.

**Postconditions**

- The workspace passes `cargo clippy --all-targets --all-features -- -D warnings`.

**Tests (must exist before implementation)**

Unit:
- `clippy_default_constructed_unit_structs_regression_is_removed`

Property:
- `workspace_clippy_gate_remains_clean_after_fix_batch`

Integration:
- `cargo_clippy_all_targets_all_features_passes`

## Scenarios

- S1: connector tests instantiate unit structs without `::default()`.
- S2: store helper items are ordered compatibly with `items_after_test_module`.
- S3: full workspace clippy gate passes after cleanup.

## Verification

- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `scripts/check-fast-feedback.sh`

## Risks and Failure Modes

- Broad lint churn could obscure the focused runtime fixes in review.
- Adding blanket `allow` attributes would hide future regressions.

## Open Questions

- Should the workspace codify a lint-specific CI target separate from fast-feedback smoke coverage?

## References

- [docs/specs/rust-workspace-bootstrap.md](/home/dikini/Projects/sharo/docs/specs/rust-workspace-bootstrap.md)
- Rust skills: `lint-workspace-lints`, `lint-rustfmt-check`, `err-no-unwrap-prod`
