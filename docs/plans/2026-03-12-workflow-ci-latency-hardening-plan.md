# Workflow CI Latency Hardening Implementation Plan

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Invariant, and Integration checks.
> - Use Property-based tests only when a generative framework is actually used.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: reduce `policy-checks` wall-clock time by removing CI workflow lint, by scoping expensive CI checks to relevant ranges, and by preserving local-first enforcement for workflow syntax and tooling-policy failures.
Architecture: keep `scripts/check-workflows.sh`, `scripts/check-fast-feedback.sh`, and `scripts/check-prepush-policy.sh` as the canonical local gates, and move CI toward range-aware confirmation rather than unconditional execution of every heavy policy surface. Bootstrap becomes the authoritative place to install a prebuilt `actionlint` binary, while CI uses commit-range path decisions to skip dependency-security and full shell-test execution when their inputs did not change.
Tech Stack: Bash scripts, Git hooks, bootstrap tooling, GitHub Actions, Bats, just.
Template-Profile: tdd-strict-v1
Updated: 2026-03-12
Status: completed

Task-Registry-Refs: TASK-WORKFLOW-CI-LATENCY-SPEC-001, TASK-WORKFLOW-CI-LATENCY-PLAN-001

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level workflow and repo-governance policies.
3. This plan's execution and verification contracts.
4. Explicit plan updates recorded in this document.

## Execution Mode

- Mode: execute-with-checkpoints
- Default: execute-with-checkpoints unless user explicitly requests plan-only output.

## Output Contract

- Remove per-push CI workflow lint and make local workflow lint bootstrap-backed and mandatory.
- Add explicit CI range gating for dependency-security and shell tests.
- Keep changes concentrated in canonical scripts, bootstrap, Bats coverage, and `policy-checks` workflow wiring.
- Record timing evidence and acceptance thresholds before implementation begins.

## Task Update Contract

- Any change that removes a CI gate must name the compensating local gate.
- Any path-based CI skip logic must be expressed in one deterministic place and be test-covered.
- Timing reductions must be measurable against the recent `policy-checks` baseline.
- Until a new remote `policy-checks` run exists after rollout, post-rollout timing evidence remains pending.

## Completion Gate

- Bootstrap installs a prebuilt `actionlint` binary locally.
- Local fast-feedback and pre-push replay still enforce workflow lint.
- `policy-checks` no longer runs dedicated workflow lint.
- CI dependency-security and shell-test steps skip for irrelevant changes and run for relevant ones.
- Shell tests, docs/task sync, and fast-feedback pass on the final tree.

## Model Compatibility Notes

- This rollout changes workflow/tooling behavior only.
- No runtime or protocol behavior changes are required.

### Task 1: Make workflow lint local-only and bootstrap-owned

**Files:**

- Modify: `scripts/bootstrap-dev.sh`
- Modify: `scripts/check-workflows.sh`
- Modify: `scripts/check-fast-feedback.sh`
- Modify: `scripts/check-prepush-policy.sh`
- Modify: `scripts/tests/test-bootstrap-dev.bats`
- Modify: `scripts/tests/test-check-workflows.bats`
- Modify: `scripts/tests/test-deterministic-workflow-gates.bats`

**Preconditions**

- The current repo already uses `scripts/check-workflows.sh` in local gates.
- Timing evidence shows CI spends about `25s` building `rhysd/actionlint`.

**Invariants**

- Workflow lint remains mandatory locally.
- Bootstrap installs a prebuilt `actionlint` binary instead of relying on source builds in CI.

**Postconditions**

- Contributors get `actionlint` through bootstrap and local gates fail clearly if the tool is unavailable.
- CI no longer pays the per-run `actionlint` step cost.

**Tests (must exist before implementation)**

Unit:
- `bootstrap_installs_prebuilt_actionlint_binary`
- `check_workflows_requires_actionlint_when_not_in_warn_missing_mode`

Invariant:
- `fast_feedback_and_prepush_call_check_workflows`

Integration:
- `policy_checks_workflow_no_longer_has_actionlint_step`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `scripts/install-bats.sh >/dev/null && "$(scripts/install-bats.sh)" scripts/tests/test-bootstrap-dev.bats scripts/tests/test-check-workflows.bats scripts/tests/test-deterministic-workflow-gates.bats`
Expected: new bootstrap/workflow-lint expectations fail before implementation.

**Implementation Steps**

1. Add bootstrap installation logic for a prebuilt `actionlint` binary.
2. Tighten `scripts/check-workflows.sh` to distinguish explicit warning mode from mandatory local enforcement.
3. Ensure local fast-feedback and pre-push replay continue to invoke workflow lint through the canonical script.
4. Remove the dedicated workflow-lint step from `.github/workflows/policy-checks.yml`.

**Green Phase (required)**

Command: `scripts/install-bats.sh >/dev/null && "$(scripts/install-bats.sh)" scripts/tests/test-bootstrap-dev.bats scripts/tests/test-check-workflows.bats scripts/tests/test-deterministic-workflow-gates.bats`
Expected: bootstrap/workflow-lint shell coverage passes.

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Targeted shell tests passing

### Task 2: Add range-based CI gating for dependency-security and shell tests

**Files:**

- Modify: `.github/workflows/policy-checks.yml`
- Modify: `scripts/tests/test-check-dependencies-security.bats`
- Modify: `scripts/tests/test-check-shell-quality.bats`
- Modify: `scripts/tests/test-justfile-targets.bats`
- Create or modify: a shell test covering CI path gating semantics if needed

**Preconditions**

- Baseline timings identify dependency-security install/run and shell tests as major runtime contributors.

**Invariants**

- Cargo input changes still run dependency-security.
- Workflow/tooling path changes still run full shell-test coverage.
- Docs-only and pure Rust-runtime changes can skip irrelevant CI lanes.

**Postconditions**

- `policy-checks` uses deterministic path/range logic to decide whether to run dependency-security and shell tests.

**Tests (must exist before implementation)**

Unit:
- `dependency_security_ci_gate_activates_only_for_cargo_inputs`
- `shell_tests_ci_gate_activates_only_for_tooling_paths`

Invariant:
- `ci_range_scope_resolution_is_shared_and_deterministic`

Integration:
- `policy_checks_skips_dependency_security_and_shell_tests_for_docs_only_range`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `scripts/install-bats.sh >/dev/null && "$(scripts/install-bats.sh)" scripts/tests/test-check-dependencies-security.bats scripts/tests/test-check-shell-quality.bats scripts/tests/test-justfile-targets.bats`
Expected: new path-gating expectations fail before workflow updates.

**Implementation Steps**

1. Add CI range classification for Cargo-impacting and tooling-impacting paths.
2. Gate dependency-security install/run on Cargo-impacting ranges.
3. Gate shell-test execution on tooling-impacting ranges.
4. Keep CI logs explicit about when steps are skipped and why.

**Green Phase (required)**

Command: `scripts/install-bats.sh >/dev/null && "$(scripts/install-bats.sh)" scripts/tests/test-check-dependencies-security.bats scripts/tests/test-check-shell-quality.bats scripts/tests/test-justfile-targets.bats`
Expected: targeted CI-path shell coverage passes.

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Targeted shell tests passing

### Task 3: Validate timing reduction and update workflow tooling docs

**Files:**

- Modify: `docs/tasks/README.md`
- Modify: `docs/tasks/tasks.csv`
- Modify: `CHANGELOG.md`
- Optionally modify: existing workflow/tooling spec docs to cross-reference the new rollout

**Preconditions**

- Implementation of Tasks 1 and 2 is complete.

**Invariants**

- Docs reflect the actual enforcement split between local gates and CI.
- Task registry rows point to the new spec/plan artifacts.

**Postconditions**

- Operator docs describe workflow lint as local-only and describe CI path-sensitive skips.
- Timing evidence after rollout is recorded against the baseline runs `23006596963` and `23005955274`.

**Tests (must exist before implementation)**

Unit:
- `docs_describe_workflow_lint_as_local_only`

Invariant:
- `scripts/doc-lint.sh --changed --strict-new`
- `scripts/check-tasks-registry.sh`

Integration:
- `scripts/check-tasks-sync.sh --changed`
- `scripts/check-fast-feedback.sh`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: current docs do not yet reflect the new local-only workflow-lint contract.

**Implementation Steps**

1. Update operator docs for local workflow lint and CI path-gating behavior.
2. Update changelog and task registry for the completed latency-hardening slice.
3. Capture before/after CI timing evidence in the completion notes.

**Green Phase (required)**

Command: `scripts/doc-lint.sh --changed --strict-new && scripts/check-tasks-registry.sh && scripts/check-tasks-sync.sh --changed && scripts/check-fast-feedback.sh`
Expected: docs, task sync, and fast-feedback pass.

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Verification commands passing
