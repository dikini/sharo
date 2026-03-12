# Workflow CI Latency Hardening

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Define tests before implementation work.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Invariant, and Integration checks.
> - Use Property-based tests only when a generative framework is actually used.
> - Run `scripts/doc-lint.sh` before commit.

Updated: 2026-03-12
Status: active
Owner: platform
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-WORKFLOW-CI-LATENCY-SPEC-001, TASK-WORKFLOW-CI-LATENCY-PLAN-001

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level workflow and verification policies.
3. This spec's CI-latency and local-gating contracts.
4. Explicit updates recorded in this document.

## Output Contract

- Reduce `policy-checks` wall-clock time without weakening the local-first failure detection model.
- Keep workflow lint local-first and remove it from per-push CI.
- Make expensive CI checks conditional on range-relevant inputs when that does not materially reduce signal.
- Preserve deterministic shell-script entrypoints as the canonical policy surfaces.

## Evidence / Verification Contract

- Timing claims must be backed by GitHub Actions job step timings.
- The spec must identify the dominant runtime contributors before proposing scope changes.
- If a CI gate is removed or narrowed, the compensating local gate must be stated explicitly.

## Model Compatibility Notes

- This slice changes workflow/tooling behavior, not Rust runtime behavior.
- The implementation should prefer existing shell entrypoints and bootstrap tooling over introducing new orchestration layers.

## Purpose

Define a follow-up hardening pass that cuts local and CI verification latency by moving workflow lint fully local, by scoping expensive CI checks to relevant changes, and by eliminating duplicated per-push validation that no longer provides proportional value.

## Scope

### In Scope

- Making `actionlint` a local-only gate installed via bootstrap as a prebuilt binary.
- Removing workflow lint from `.github/workflows/policy-checks.yml`.
- Conditional CI dependency-security execution based on Cargo input changes.
- Conditional CI shell-test execution based on workflow/tooling path changes.
- Preserving local fast-feedback and pre-push replay as the primary enforcement surfaces for workflow lint.

### Out of Scope

- Removing local workflow lint from fast-feedback or pre-push replay.
- Removing dependency-security, shell tests, or workflow lint from the repository entirely.
- Broad runtime, protocol, or daemon test behavior changes unrelated to verification latency.

## Core Terms

- `Workflow Lint Locality`: policy that `actionlint` runs locally via canonical scripts and hooks, not in per-push CI.
- `CI Latency Hardening`: reducing wall-clock time spent in `policy-checks` while preserving meaningful failure detection.
- `Cargo-Scoped Dependency Security`: running dependency/audit checks only when `Cargo.toml` or `Cargo.lock` changed in the push/PR range.
- `Tooling-Scoped Shell Tests`: running full shell-contract coverage only when workflow/tooling files changed.

## Interfaces / Contracts

- `scripts/check-workflows.sh` remains the canonical workflow-lint entrypoint.
- `scripts/bootstrap-dev.sh` installs a prebuilt `actionlint` binary and treats missing installation as a local bootstrap/setup failure.
- `scripts/check-fast-feedback.sh` and `scripts/check-prepush-policy.sh` continue to call `scripts/check-workflows.sh`.
- `.github/workflows/policy-checks.yml` does not run `actionlint` as a dedicated CI step.
- `.github/workflows/policy-checks.yml` gates dependency-security on push/PR ranges that modify `Cargo.toml` or `Cargo.lock`.
- `.github/workflows/policy-checks.yml` gates shell tests on workflow/tooling path relevance instead of always running the full shell suite.

## Invariants

- Workflow syntax issues must still be caught before push through local bootstrap and local gates.
- CI remains a confirmation/backstop layer, but it should not pay the `actionlint` cost on every run.
- Expensive CI checks should only run when their input surfaces changed.
- Canonical shell entrypoints remain the only place where workflow-check semantics are encoded.
- Step-timing evidence must justify any removed or narrowed CI gate.

## Task Contracts

### Task 1: Define local-only workflow lint enforcement

**Preconditions**

- Local fast-feedback and pre-push replay are already active.
- `scripts/check-workflows.sh` already exists as the canonical workflow-lint entrypoint.

**Invariants**

- Workflow lint remains mandatory locally.
- Bootstrap owns installation of the required `actionlint` binary.

**Postconditions**

- The spec defines workflow lint as local-only for per-push enforcement and removes it from per-push CI.

**Tests (must exist before implementation)**

Unit:
- `bootstrap_installs_prebuilt_actionlint_binary`
- `check_workflows_fails_when_actionlint_missing_after_bootstrap_contract_is_enabled`

Invariant:
- `fast_feedback_and_prepush_still_call_check_workflows`

Integration:
- `policy_checks_no_longer_runs_dedicated_actionlint_step`

Property-based (optional):
- not applicable

### Task 2: Define timing-driven CI scope reductions

**Preconditions**

- Step timings from recent `policy-checks` runs are available.

**Invariants**

- Conditional CI scope must be derived from range-based file changes.
- Narrowed checks must have explicit compensating local gates where applicable.

**Postconditions**

- The spec defines Cargo-scoped dependency-security and tooling-scoped shell-test execution in CI.

**Tests (must exist before implementation)**

Unit:
- `dependency_security_ci_gate_activates_only_for_cargo_inputs`
- `shell_tests_ci_gate_activates_only_for_tooling_paths`

Invariant:
- `ci_range_resolution_feeds_scope_decisions_deterministically`

Integration:
- `policy_checks_skips_dependency_security_and_shell_tests_for_docs_only_changes`

Property-based (optional):
- not applicable

### Task 3: Define evidence-based rollout and regression expectations

**Preconditions**

- Baseline timing evidence has been recorded for recent successful and failed CI runs.

**Invariants**

- The rollout must preserve local-first workflow lint detection.
- CI runtime reductions must be measurable after rollout.

**Postconditions**

- The spec defines acceptance criteria in terms of both correctness and elapsed-time reduction.

**Tests (must exist before implementation)**

Unit:
- `timing_evidence_captured_in_docs_for_recent_policy_runs`

Invariant:
- `acceptance_criteria_require_local_workflow_lint_and_reduced_ci_wall_clock`

Integration:
- `post_rollout_policy_checks_wall_clock_drops_for_non_cargo_non_tooling_changes`

Property-based (optional):
- not applicable

## Scenarios

- S1: A contributor edits `.github/workflows/policy-checks.yml`; local bootstrap has installed `actionlint`, and fast-feedback/pre-push fail locally without involving CI.
- S2: A contributor pushes a docs-only change; CI skips dependency-security and full shell-test coverage because Cargo and tooling inputs did not change.
- S3: A contributor changes `Cargo.lock`; CI runs dependency-security because the push touched Cargo inputs.
- S4: A contributor pushes from an environment without local workflow lint setup; a broken workflow may land, but GitHub Actions immediately surfaces the problem, which is accepted as a low-frequency residual risk.

## Timing Evidence

- Successful run `23006596963` (`759fa57`): total job time about `9m54s`.
- Failed run `23005955274` (`9e82dd0`): total job time about `1m31s`, failing inside the canonical verification entrypoint.
- Dominant successful-run step costs:
  - `Install dependency security tools`: `321s`
  - `Run canonical verification entrypoint`: `55s`
  - `Run shell tests`: `55s`
  - `Run Rust workspace tests`: `51s`
  - `Build rhysd/actionlint@v1.7.11`: `25s`
  - `Install shell quality tools`: `14s`
  - `Run flaky regression replay when daemon paths changed`: `12s`
  - `Install nightly toolchain for fuzzing`: `11s`
  - `Run property test profile`: `11s`

## Verification

- `scripts/doc-lint.sh --changed --strict-new`
- `scripts/check-tasks-registry.sh`
- `scripts/check-tasks-sync.sh --changed`
- `scripts/check-fast-feedback.sh`

## Risks and Failure Modes

- A broken workflow can still reach GitHub if local workflow lint is bypassed; this is accepted because workflow files change rarely and CI will fail immediately.
- If bootstrap fails to install `actionlint` reliably, local workflow lint becomes noisy and easy to bypass.
- Over-aggressive CI skipping can hide regressions if path filters are too narrow or drift from actual tool ownership boundaries.

## Open Questions

- Should the same timing-driven approach later split `policy-checks` into parallel jobs, or is scope reduction sufficient for now?
- Should nightly/toolchain-dependent checks move into separate scheduled workflows after the first latency pass lands?

## References

- [local-policy-replay-hardening.md](local-policy-replay-hardening.md)
- [policy-checks run 23006596963](https://github.com/dikini/sharo/actions/runs/23006596963)
- [policy-checks run 23005955274](https://github.com/dikini/sharo/actions/runs/23005955274)
