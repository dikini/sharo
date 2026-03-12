# Local Policy Replay Hardening Implementation Plan

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Invariant, and Integration checks.
> - Use Property-based tests only when a generative framework is actually used.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: catch policy-check failures locally before `git push` by adding a blocking pre-push replay, a docs-portability guard, and a path-sensitive daemon regression replay.
Architecture: keep `pre-commit` optimized for changed-scope feedback and add a separate blocking `pre-push` replay for full/range-based policy surfaces. Reuse canonical shell scripts and hook wiring so local and CI logic stay aligned while docs portability and flaky daemon replay remain bounded and explicit.
Tech Stack: Bash scripts, Git hooks, Bats, GitHub Actions, just, existing daemon regression tests.
Template-Profile: tdd-strict-v1
Updated: 2026-03-12
Status: completed

Task-Registry-Refs: TASK-LOCAL-POLICY-REPLAY-SPEC-001, TASK-LOCAL-POLICY-REPLAY-PLAN-001

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level workflow and repo-governance policies.
3. This plan's execution and verification contracts.
4. Explicit plan updates recorded in this document.

## Execution Mode

- Mode: execute-with-checkpoints
- Default: execute-with-checkpoints unless user explicitly requests plan-only output.

## Output Contract

- Add one blocking `pre-push` replay entrypoint and hook.
- Add one cheap docs-portability script reusable from pre-commit, pre-push, and CI range checks.
- Add one path-sensitive flaky daemon regression replay script.
- Keep local gates deterministic and avoid passing non-markdown files into markdown-only linters.

## Task Update Contract

- New workflow checks must reuse canonical scripts, not duplicate logic inline in hooks.
- New path-sensitive skips must remain explicit and test-covered.
- Any new docs or script changes must update `docs/tasks/tasks.csv` and `CHANGELOG.md`.

## Completion Gate

- Shell tests for pre-push/doc-portability/flaky replay pass.
- Docs/task sync and strict docs lint pass.
- `scripts/check-fast-feedback.sh` passes on the final tree.

## Model Compatibility Notes

- This slice does not change daemon runtime behavior; it hardens local workflow detection around existing tests and scripts.
- If future changes introduce Rust helpers, they must keep shell entrypoints stable.

### Task 1: Add failing shell tests for local pre-push replay

**Files:**

- Create: `scripts/tests/test-prepush-policy.bats`
- Modify: `scripts/tests/test-precommit-fast-feedback.bats`
- Modify: `scripts/tests/test-justfile-targets.bats`

**Preconditions**

- Existing Bats harness and deterministic workflow gate tests are passing.

**Invariants**

- Tests define hook delegation, range resolution, docs portability, dependency-security scoping, and flaky replay behavior before implementation.
- Pre-commit tests keep working with any new script dependencies.

**Postconditions**

- New tests fail before the new scripts and hook wiring exist.

**Tests (must exist before implementation)**

Unit:
- `pre_push_policy_uses_upstream_range_when_tracking_branch_exists`
- `pre_push_policy_falls_back_to_origin_main_when_no_upstream_exists`
- `doc_portability_rejects_machine_local_and_worktree_local_paths`

Invariant:
- `pre_push_hook_delegates_to_prepush_policy_script`

Integration:
- `pre_push_policy_runs_dependency_checks_only_when_cargo_inputs_change`
- `flaky_regressions_skip_unrelated_changes_and_run_for_daemon_paths`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `scripts/install-bats.sh >/dev/null && "$(scripts/install-bats.sh)" scripts/tests/test-prepush-policy.bats scripts/tests/test-precommit-fast-feedback.bats scripts/tests/test-justfile-targets.bats`
Expected: fails because `pre-push` hook and new workflow scripts do not exist yet.

**Implementation Steps**

1. Add the new Bats file covering hook delegation, range resolution, docs portability, dependency scoping, and flaky replay.
2. Expand pre-commit and justfile shell tests for new script dependencies/targets.
3. Verify the new tests fail for the expected missing-script reasons before implementation.

**Green Phase (required)**

Command: `scripts/install-bats.sh >/dev/null && "$(scripts/install-bats.sh)" scripts/tests/test-prepush-policy.bats scripts/tests/test-precommit-fast-feedback.bats scripts/tests/test-justfile-targets.bats`
Expected: all targeted shell tests pass.

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Targeted shell tests passing

### Task 2: Implement local pre-push replay, docs portability, and flaky replay scripts

**Files:**

- Create: `scripts/check-prepush-policy.sh`
- Create: `scripts/check-doc-portability.sh`
- Create: `scripts/check-flaky-regressions.sh`
- Create: `.githooks/pre-push`
- Modify: `.githooks/pre-commit`
- Modify: `scripts/check-fast-feedback.sh`
- Modify: `.github/workflows/policy-checks.yml`
- Modify: `justfile`

**Preconditions**

- Failing shell tests define the required behavior.

**Invariants**

- `pre-commit` remains changed-scope and cheap.
- `pre-push` blocks pushes on local replay failures.
- Range-based markdown checks filter markdown files only.
- Dependency-security replay is scoped to Cargo input changes.
- Flaky daemon replay is scoped to daemon-impacting path changes.

**Postconditions**

- Blocking pre-push replay exists and uses upstream-range semantics.
- Cheap docs portability checks run locally and in CI range validation.
- Path-sensitive daemon regression replay exists for pre-push and CI backstop use.
- `just` exposes explicit operator entry points for the new surfaces.

**Tests (must exist before implementation)**

Unit:
- `pre_push_hook_delegates_to_prepush_policy_script`
- `doc_portability_rejects_machine_local_and_worktree_local_paths`

Invariant:
- `pre_push_policy_runs_dependency_checks_only_when_cargo_inputs_change`
- `flaky_regressions_skip_unrelated_changes_and_run_for_daemon_paths`

Integration:
- `policy_checks_runs_doc_portability_and_flaky_replay_backstops`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `scripts/install-bats.sh >/dev/null && "$(scripts/install-bats.sh)" scripts/tests/test-prepush-policy.bats`
Expected: fails until the new scripts and hook exist.

**Implementation Steps**

1. Add `scripts/check-doc-portability.sh` with `--changed`, `--range`, and `--path` support over canonical markdown docs scope.
2. Add `scripts/check-flaky-regressions.sh` with path-sensitive skips and repeated daemon test execution.
3. Add `scripts/check-prepush-policy.sh` with upstream/fallback range resolution, full local replay, conditional dependency checks, and range-based docs/task/changelog checks.
4. Add `.githooks/pre-push` delegation and wire docs portability into `pre-commit` plus `check-fast-feedback.sh`.
5. Add `doc-portability`, `flaky-regressions`, and `prepush-policy` `just` targets.
6. Add CI backstop steps for docs portability and flaky replay in `policy-checks`.

**Green Phase (required)**

Command: `scripts/install-bats.sh >/dev/null && "$(scripts/install-bats.sh)" scripts/tests/test-prepush-policy.bats scripts/tests/test-precommit-fast-feedback.bats scripts/tests/test-justfile-targets.bats`
Expected: targeted hook/script tests pass.

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Hook/script tests passing

### Task 3: Document the new workflow-hardening contract and complete repo governance updates

**Files:**

- Create: `docs/specs/local-policy-replay-hardening.md`
- Create: `docs/plans/2026-03-12-local-policy-replay-hardening-plan.md`
- Modify: `docs/tasks/README.md`
- Modify: `docs/tasks/tasks.csv`
- Modify: `CHANGELOG.md`

**Preconditions**

- Script and hook behavior is implemented and test-covered.

**Invariants**

- New docs follow `tdd-strict-v1`.
- Task registry rows reference existing source files and contain matching task ids.
- Operator docs describe the new local replay behavior without weakening existing governance requirements.

**Postconditions**

- The repo has a canonical spec and plan for local workflow hardening.
- Commands and usage guidance document the new pre-push, docs-portability, and flaky-replay surfaces.
- Changelog and task registry reflect the completed slice.

**Tests (must exist before implementation)**

Unit:
- `docs_reference_local_policy_replay_hardening_contract`

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
Expected: no new local-policy-replay docs exist yet.

**Implementation Steps**

1. Add the new spec and completed implementation plan documenting blocking pre-push, docs portability, and path-sensitive flaky replay.
2. Update `docs/tasks/README.md` command/use guidance for the new workflow surfaces.
3. Add task registry rows and changelog entry for the completed hardening slice.

**Green Phase (required)**

Command: `scripts/doc-lint.sh --changed --strict-new && scripts/check-tasks-registry.sh && scripts/check-tasks-sync.sh --changed && scripts/check-fast-feedback.sh`
Expected: docs, task registry/sync, and fast-feedback pass.

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Docs/task/fast-feedback verification passing
