# Workflow Tooling Rollout Implementation Plan

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Invariant, and Integration checks.
> - Use Property-based tests only when a generative framework is actually used.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: roll out six high-impact workflow tools in phased, deterministic slices without destabilizing existing verification flows.
Architecture: implement script and CI changes in small independent tasks with explicit red/green checks, beginning with additive non-breaking paths, then graduate to required checks after burn-in. Keep runtime-focused additions (`proptest`, `loom`) scoped to high-risk modules and bounded execution profiles.
Tech Stack: Rust 2024, Bash scripts, Bats tests, GitHub Actions, cargo-nextest, cargo-deny, cargo-audit, just, proptest, loom.
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-WORKFLOW-TOOLING-PLAN-001, TASK-WORKFLOW-TOOLING-SPEC-001

---

## Instruction Priority

1. System and developer workflow constraints.
2. This plan's rollout order and verification gates.
3. Existing workflow scripts, tests, and CI jobs referenced by each task.

## Output Contract

- Keep workflow-tooling rollout behavior aligned with the canonical spec.
- Preserve bounded generative testing while using deterministic replay only for captured regressions.
- Maintain script-first and CI-first verification entrypoints documented in this plan.

## Model Compatibility Notes

- This plan governs workflow scripts, CI configuration, and bounded test coverage rather than Rust runtime behavior.
- Generative testing guidance should prefer exploratory runs by default and explicit deterministic replay for known failures.

## Execution Mode

- Execute tasks in the documented order unless a review finding requires a narrower corrective patch.
- Re-run the named red/green commands for the touched task before completion claims.

## Task Update Contract

- Update `CHANGELOG.md` for task-completion work.
- Keep `docs/tasks/tasks.csv` synchronized whenever this plan or its owned scripts change.
- Record seed-policy or replay-policy changes in the relevant workflow docs when generative testing behavior changes.

## Completion Gate

- `scripts/doc-lint.sh --changed --strict-new`
- `scripts/check-tasks-sync.sh --changed`
- Task-scoped shell or Rust verification listed in the touched task
- Any higher-level fast-feedback or pre-push gate required by repository policy

### Task 1: Add `cargo-nextest` test path

**Files:**

- Create: `scripts/check-tests.sh`
- Create: `scripts/tests/test-check-tests.bats`
- Modify: `scripts/check-fast-feedback.sh`
- Modify: `docs/tasks/README.md`

**Preconditions**

- Baseline `cargo test --workspace` passes.

**Invariants**

- New script falls back to `cargo test` when `nextest` is unavailable.
- Exit code parity is preserved for pass/fail outcomes.

**Postconditions**

- Fast feedback can use `nextest` path with deterministic fallback.

**Tests (must exist before implementation)**

Unit:
- `test_check_tests_prefers_nextest_when_available`

Invariant:
- `test_check_tests_falls_back_to_cargo_test`

Integration:
- `test_fast_feedback_uses_check_tests_wrapper`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `bats scripts/tests/test-check-tests.bats`
Expected: fails because `check-tests.sh` does not exist.

**Implementation Steps**

1. Add failing Bats file for `nextest` preference and fallback behavior.
2. Implement `scripts/check-tests.sh` with deterministic command selection.
3. Replace direct Rust test invocation in `check-fast-feedback.sh` with wrapper.
4. Document usage in `docs/tasks/README.md`.

**Green Phase (required)**

Command: `bats scripts/tests/test-check-tests.bats && scripts/check-fast-feedback.sh`
Expected: wrapper tests pass and fast-feedback remains green.

**Refactor Phase (optional but controlled)**

Allowed scope: `scripts/check-tests.sh`, `scripts/tests/test-check-tests.bats`
Re-run: `scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 2: Add merge-result CI required gate

**Files:**

- Create: `.github/workflows/merge-result-gate.yml`
- Create: `scripts/check-merge-result.sh`
- Create: `scripts/tests/test-check-merge-result.bats`
- Modify: `docs/tasks/README.md`

**Preconditions**

- Existing CI policy checks are green.

**Invariants**

- Workflow evaluates merged ref and not only branch head.
- Gate uses repository canonical scripts.

**Postconditions**

- Merge-result validation appears as required status for integration branches.

**Tests (must exist before implementation)**

Unit:
- `test_check_merge_result_invokes_required_local_gates`

Invariant:
- `test_merge_result_workflow_uses_merge_ref_context`

Integration:
- `test_merge_result_ci_job_runs_on_pull_request_events`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `bats scripts/tests/test-check-merge-result.bats`
Expected: fails because script/workflow do not exist.

**Implementation Steps**

1. Add failing Bats checks for script behavior and workflow trigger shape.
2. Add `scripts/check-merge-result.sh` reusing existing deterministic gate scripts.
3. Add CI workflow and mark as required in branch protection policy documentation.

**Green Phase (required)**

Command: `bats scripts/tests/test-check-merge-result.bats`
Expected: Bats checks pass.

**Refactor Phase (optional but controlled)**

Allowed scope: merge-result workflow/script files only
Re-run: `scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 3: Add dependency governance and security gates (`cargo-deny`, `cargo-audit`)

**Files:**

- Create: `deny.toml`
- Create: `audit.toml` (or equivalent policy config used by chosen audit invocation)
- Create: `scripts/check-dependencies-security.sh`
- Create: `scripts/tests/test-check-dependencies-security.bats`
- Modify: `.github/workflows/policy-checks.yml`

**Preconditions**

- Team agrees baseline exceptions policy.

**Invariants**

- Local and CI security checks use the same script.
- Initial rollout supports warn-only mode flag for controlled adoption.

**Postconditions**

- Dependency and security policy checks are enforceable and deterministic.

**Tests (must exist before implementation)**

Unit:
- `test_dependency_security_script_detects_missing_tool_binaries`

Invariant:
- `test_dependency_security_script_enforces_strict_mode`

Integration:
- `test_policy_checks_workflow_invokes_dependency_security_script`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `bats scripts/tests/test-check-dependencies-security.bats`
Expected: fails because script/config files are missing.

**Implementation Steps**

1. Add failing script tests for tool detection and strict mode behavior.
2. Add policy configs and `check-dependencies-security.sh`.
3. Wire script into CI workflow with phased strictness toggle.

**Green Phase (required)**

Command: `bats scripts/tests/test-check-dependencies-security.bats`
Expected: script tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: dependency security scripts/config/workflow only
Re-run: `scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 4: Add deterministic task runner entry points (`just`)

**Files:**

- Create: `justfile`
- Create: `scripts/tests/test-justfile-targets.bats`
- Modify: `README.md`
- Modify: `.github/workflows/policy-checks.yml`

**Preconditions**

- Script-based checks from prior tasks are available.

**Invariants**

- `just` targets remain thin wrappers around canonical scripts.
- CI and local docs stay aligned on command entry points.

**Postconditions**

- `just verify`, `just fast-feedback`, `just merge-gate`, and `just daemon-invariants` are available and documented.

**Tests (must exist before implementation)**

Unit:
- `test_justfile_includes_required_targets`

Invariant:
- `test_just_verify_target_maps_to_check_fast_feedback`

Integration:
- `test_ci_invokes_just_verify_target`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `bats scripts/tests/test-justfile-targets.bats`
Expected: fails before `justfile` exists.

**Implementation Steps**

1. Add failing Bats target-map tests.
2. Add `justfile` with deterministic wrapper targets.
3. Update docs and CI invocation strategy.

**Green Phase (required)**

Command: `bats scripts/tests/test-justfile-targets.bats`
Expected: all target mapping tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `justfile` and mapping tests only
Re-run: `scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 5: Add `proptest` invariants for protocol and idempotency

**Files:**

- Modify: `crates/sharo-core/Cargo.toml`
- Modify: `crates/sharo-core/tests/protocol_tests.rs`
- Create: `crates/sharo-daemon/tests/idempotency_properties.rs`
- Modify: `.github/workflows/policy-checks.yml`

**Preconditions**

- Property-test runtime budget and regression-replay guidance are documented.

**Invariants**

- Property tests are bounded in CI via explicit case-count/runtime limits.
- Failing generative runs can be replayed deterministically via captured seeds or minimized inputs.
- Existing example-based tests remain unchanged unless redundancy is intentional.

**Postconditions**

- Property-based checks guard roundtrip/protocol and idempotency contracts.

**Tests (must exist before implementation)**

Unit:
- `prop_protocol_roundtrip_preserves_task_summary_fields`

Invariant:
- `prop_idempotency_replay_never_double_executes`

Integration:
- `test_policy_checks_runs_property_test_profile`

Property-based (optional):
- required in this task

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-core --test protocol_tests prop_protocol_roundtrip_preserves_task_summary_fields`
Expected: fails because proptest case is missing.

**Implementation Steps**

1. Add failing property tests in core and daemon test modules.
2. Add proptest dependency and bounded config (case cap plus explicit failure replay guidance).
3. Wire property tests into CI in a bounded profile.

**Green Phase (required)**

Command: `cargo test -p sharo-core --test protocol_tests && cargo test -p sharo-daemon --test idempotency_properties`
Expected: property suites pass within bounded runtime.

**Refactor Phase (optional but controlled)**

Allowed scope: property tests and test dependency config only
Re-run: `scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 6: Add `loom` model checks for critical concurrency logic

**Files:**

- Modify: `crates/sharo-daemon/Cargo.toml`
- Create: `crates/sharo-daemon/tests/loom_submit_shutdown.rs`
- Modify: `crates/sharo-daemon/src/store.rs`
- Modify: `crates/sharo-daemon/src/main.rs`
- Modify: `.github/workflows/policy-checks.yml`

**Preconditions**

- Concurrency-critical code boundaries are identifiable and extractable for model tests.

**Invariants**

- Loom tests stay narrowly scoped and avoid full-daemon integration simulation.
- Modeled contracts reflect production invariants (reservation release, handler drain).

**Postconditions**

- A loom suite validates key interleavings for idempotency reservation release and shutdown handler drain.

**Tests (must exist before implementation)**

Unit:
- `loom_submit_reservation_release_on_commit_failure`
- `loom_shutdown_drain_preserves_accepted_handler_completion`

Invariant:
- `loom_duplicate_submit_never_double_executes_provider`

Integration:
- `test_policy_checks_runs_loom_profile`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-daemon --test loom_submit_shutdown -- --nocapture`
Expected: fails because loom test target does not exist.

**Implementation Steps**

1. Extract minimal state-machine units required for loom modeling.
2. Add failing loom model tests for reservation-release and shutdown-drain semantics.
3. Implement minimal synchronization refinements if model checks fail.
4. Add CI profile for loom tests (nightly/opt-in first, then required by policy decision).

**Green Phase (required)**

Command: `cargo test -p sharo-daemon --test loom_submit_shutdown -- --nocapture`
Expected: loom model tests pass with bounded execution budget.

**Refactor Phase (optional but controlled)**

Allowed scope: extracted state-machine units and loom tests only
Re-run: `scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated
