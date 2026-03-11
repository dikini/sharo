# Workflow Tooling Rollout

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Define tests before implementation work.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Invariant, and Integration checks.
> - Use Property-based tests only when a generative framework is actually used.
> - Run `scripts/doc-lint.sh` before commit.

Updated: 2026-03-06
Status: active
Owner: platform
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-WORKFLOW-TOOLING-SPEC-001, TASK-WORKFLOW-TOOLING-PLAN-001

## Instruction Priority

1. Follow repository policy and task-registry contracts.
2. Keep rollout deterministic and reproducible locally and in CI.
3. Prefer narrow, additive gate wiring over broad refactors.

## Output Contract

- Required outputs are policy scripts, CI wiring, and tests that prove gate behavior.
- All added gates must be invocable from deterministic local command surfaces.
- Changes must preserve existing `scripts/check-fast-feedback.sh` reliability.

## Model Compatibility Notes

- This spec uses plain-language deterministic gate contracts compatible with GPT-5 class coding agents.
- Tool and script names are canonical and must remain stable across automated edits.

## Evidence / Verification Contract

- Provide command evidence for:
  - `scripts/check-fast-feedback.sh`
  - `scripts/check-tests.sh --workspace`
  - dependency security and workflow gate checks where touched
- Task closure requires `docs/tasks/tasks.csv` and `CHANGELOG.md` updates.

## Purpose

Define an incremental tooling rollout that improves deterministic delivery and guardrail coverage across local development and CI for six priority additions: `cargo-nextest`, merge-result CI gating, `cargo-deny`, `cargo-audit`, task runner standardization, `proptest`, and `loom`.

## Scope

### In Scope

- Tooling integration strategy, enforcement boundaries, and rollback safety.
- Deterministic command surfaces for local and CI workflows.
- Invariant-focused verification for concurrency and idempotency risk paths.

### Out of Scope

- Runtime behavior changes to daemon or protocol, except for test harness extraction needed by `loom` and `proptest`.
- Non-Rust dependency management beyond required CI hooks.

## Core Terms

- `Tooling Gate`: required check that can block commit or merge.
- `Merge-Result Verification`: checks run on the exact merged tree.
- `Deterministic Entry Point`: one canonical command surface that local and CI both use.
- `Invariant Test`: test asserting behavioral contracts (retry, idempotency, shutdown drain).

## Interfaces / Contracts

- `scripts/check-fast-feedback.sh` remains the canonical local quality wrapper.
- Add `justfile` (or `mise` tasks) as a stable command interface, initially non-breaking and additive.
- CI must expose separate required statuses for:
  - merge-result checks
  - dependency governance/security checks
  - runtime invariant checks
- New tooling must begin in observe/warn mode where risk of false positives is non-trivial, then move to required status after burn-in.

## Invariants

- Any “green” claim for merge readiness must include merge-result evidence.
- Dependency and security policy checks are deterministic and reproducible locally.
- Runtime invariants remain covered by dedicated narrow checks, not only broad workspace tests.
- Tooling adoption must not reduce existing `check-fast-feedback` reliability.

## Task Contracts

### Task 1: Introduce `cargo-nextest` as primary fast test runner

**Preconditions**

- Existing test suites pass under `cargo test`.

**Invariants**

- `nextest` and `cargo test` report equivalent pass/fail outcomes for targeted suites.

**Postconditions**

- Fast-feedback path can run via `nextest` with measured latency improvement.

**Tests (must exist before implementation)**

Unit:
- `test_check_tests_prefers_nextest_when_available`

Invariant:
- `test_nextest_and_cargo_test_exit_code_parity`

Integration:
- `test_fast_feedback_with_nextest_path`

Property-based (optional):
- not applicable

### Task 2: Add merge-result required CI gate (and optional queue)

**Preconditions**

- CI workflow supports merge refs or queue-managed merge commits.

**Invariants**

- Gate evaluates exact merged tree, not isolated branch head only.

**Postconditions**

- Merge cannot complete when merge-result compatibility fails.

**Tests (must exist before implementation)**

Unit:
- `test_merge_result_workflow_invokes_required_scripts`

Invariant:
- `test_merge_result_job_uses_merge_ref`

Integration:
- `test_merge_result_gate_blocks_protocol_shape_regression`

Property-based (optional):
- not applicable

### Task 3: Add `cargo-deny` and `cargo-audit` dependency/security gates

**Preconditions**

- Policy config files are reviewed and agreed.

**Invariants**

- Policy findings are reproducible locally and in CI.

**Postconditions**

- Dependency/license/security regressions are surfaced before merge.

**Tests (must exist before implementation)**

Unit:
- `test_deny_config_parses`
- `test_audit_config_parses`

Invariant:
- `test_security_gate_nonzero_on_known_vulnerability_fixture`

Integration:
- `test_ci_security_jobs_run_on_rust_changes`

Property-based (optional):
- not applicable

### Task 4: Add deterministic task-runner entry points (`just` preferred)

**Preconditions**

- Existing scripts are stable and independently runnable.

**Invariants**

- Task runner targets delegate to existing scripts without semantic drift.

**Postconditions**

- Developers and CI use one canonical command map for validation flows.

**Tests (must exist before implementation)**

Unit:
- `test_just_verify_target_maps_to_fast_feedback`

Invariant:
- `test_just_merge_gate_target_maps_to_merge_compat_checks`

Integration:
- `test_ci_invokes_just_targets`

Property-based (optional):
- not applicable

### Task 5: Add `proptest` coverage for protocol and idempotency invariants

**Preconditions**

- Deterministic seed policy and CI runtime bounds are defined.

**Invariants**

- Property tests remain bounded and deterministic in CI.

**Postconditions**

- Invariant coverage extends beyond hand-picked examples for protocol roundtrip and idempotency replay behavior.

**Tests (must exist before implementation)**

Unit:
- `prop_protocol_roundtrip_preserves_task_summary_fields`

Invariant:
- `prop_idempotency_replay_never_transitions_to_double_execution`

Integration:
- `test_property_suite_runs_in_ci_profile`

Property-based (optional):
- `proptest` required for this task

### Task 6: Add `loom` model checks for critical concurrency state machines

**Preconditions**

- Concurrency-critical logic is extracted into loom-testable units.

**Invariants**

- No lock ordering or stale reservation behavior violates modeled safety contracts under explored schedules.

**Postconditions**

- `loom` suite runs in CI (possibly nightly first, then required for touched modules).

**Tests (must exist before implementation)**

Unit:
- `loom_submit_reservation_release_on_commit_failure`
- `loom_shutdown_drain_does_not_drop_accepted_connection`

Invariant:
- `loom_duplicate_submit_never_double_executes_provider`

Integration:
- `test_loom_job_runs_in_ci`

Property-based (optional):
- not applicable

## Scenarios

- S1: branch passes local tests but merge-result gate fails due to shape drift.
- S2: new vulnerable crate version is introduced; security gate blocks integration.
- S3: runtime idempotency bug slips through examples; property test catches it.
- S4: concurrency race appears under rare interleaving; loom model test catches it pre-merge.

## Verification

- `scripts/doc-lint.sh --changed --strict-new`
- `scripts/check-fast-feedback.sh`
- `scripts/check-tasks-registry.sh`
- `scripts/check-tasks-sync.sh --changed`

## Risks and Failure Modes

- CI duration may increase if rollout is unphased.
- `loom` tests can be expensive without strict scope control.
- security checks may initially create false-positive noise if policy baselines are not tuned.

## Open Questions

- Should `loom` remain nightly-only for a period before making it a required status for touched modules?
- Is `just` acceptable as a mandatory developer dependency, or should task runner use `mise` for broader environment standardization?

## References

- [deterministic-workflow-hardening.md](/home/dikini/Projects/sharo/docs/specs/deterministic-workflow-hardening.md)
- Rust skills: `test-proptest-properties`, `async-no-lock-await`, `err-result-over-panic`, `lint-workspace-lints`
