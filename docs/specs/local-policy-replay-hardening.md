# Local Policy Replay Hardening

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

Task-Registry-Refs: TASK-LOCAL-POLICY-REPLAY-SPEC-001, TASK-LOCAL-POLICY-REPLAY-PLAN-001

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level workflow and verification policies.
3. This spec's local-gate contracts and invariants.
4. Explicit updates recorded in this document.

## Output Contract

- Add a blocking local `pre-push` replay that catches the policy-check classes recently escaping to CI.
- Keep `pre-commit` fast and changed-scope.
- Reuse canonical repo scripts instead of duplicating CI logic in hooks.
- Preserve CI `policy-checks` as a backstop, not the first discovery point.

## Evidence / Verification Contract

- Every completion claim must cite shell-test and fast-feedback evidence.
- New workflow guards must be runnable locally without changing repo-tracked state.
- If a gate is range-based in CI, the local replay must exercise the same range semantics.

## Model Compatibility Notes

- New workflow hardening is implemented in shell and hook tooling, not in Rust runtime code.
- If Rust helper code is introduced later, it must remain thin and subordinate to the existing shell-entrypoint workflow.

## Purpose

Define deterministic local workflow gates that catch policy-check failures before `git push`, with special focus on docs portability, range-based docs/task/changelog checks, dependency-security drift, and historically unstable daemon regressions.

## Scope

### In Scope

- Blocking `.githooks/pre-push` workflow replay.
- Canonical `scripts/check-prepush-policy.sh` range-based gate.
- Canonical `scripts/check-doc-portability.sh` for machine-local/worktree-local docs references.
- Canonical `scripts/check-flaky-regressions.sh` for path-sensitive repeated daemon regression replay.
- `just` entry points and operator docs for the new local gates.
- CI backstop wiring for docs portability and flaky regression replay.

### Out of Scope

- Removing CI `policy-checks`.
- Replacing `pre-commit` fast-feedback flow with full pre-push replay.
- Broad runtime or protocol behavior changes in daemon/core crates.

## Core Terms

- `Pre-Push Policy Replay`: blocking local gate that replays expensive/range-based policy surfaces before a push is accepted.
- `Docs Portability Gate`: deterministic rejection of workstation-specific or worktree-specific references in canonical docs.
- `Flaky Regression Replay`: repeated execution of known high-risk daemon tests when relevant paths changed.
- `Upstream Range`: `@{upstream}...HEAD` when a tracking branch exists, otherwise `origin/main...HEAD`.

## Interfaces / Contracts

- `.githooks/pre-push` is a blocking hook and delegates to `scripts/check-prepush-policy.sh`.
- `scripts/check-prepush-policy.sh`:
  - resolves upstream range deterministically
  - runs `scripts/check-fast-feedback.sh --all --no-marker`
  - replays full-scope shell/workflow quality checks
  - runs range-based docs portability, docs lint, docs terminology, sync-manifest, task-sync, conventional-commit, and changelog checks
  - runs dependency-security checks only when `Cargo.toml` or `Cargo.lock` changed in range
  - runs flaky daemon regression replay only when daemon-impacting paths changed in range
- `scripts/check-doc-portability.sh` supports `--changed`, `--range <git-range>`, `--path <file>`, and full-scope execution.
- `scripts/check-flaky-regressions.sh` supports `--changed`, `--range <git-range>`, `--all`, and `--iterations <count>`.
- `just` exposes:
  - `prepush-policy`
  - `doc-portability`
  - `flaky-regressions`

## Invariants

- `pre-commit` remains cheaper than `pre-push`; it does not absorb the full replay burden.
- Pushes are blocked locally when policy-replay checks fail.
- Canonical docs must not contain workstation-only absolute paths or local worktree references.
- Range-based local replay must not feed non-markdown files into markdown-only lint/terminology tools.
- Flaky regression replay remains path-sensitive so unrelated pushes do not pay the daemon-test replay cost.

## Task Contracts

### Task 1: Define the blocking local pre-push replay contract

**Preconditions**

- Existing `pre-commit` and fast-feedback gates are active.
- Existing CI `policy-checks` workflow remains canonical.

**Invariants**

- Upstream range resolution is deterministic.
- Local replay remains a hook-level backstop, not a replacement for CI.

**Postconditions**

- The spec defines a blocking local replay surface with exact scope and fallback behavior.

**Tests (must exist before implementation)**

Unit:
- `pre_push_policy_uses_upstream_range_when_tracking_branch_exists`
- `pre_push_policy_falls_back_to_origin_main_when_no_upstream_exists`

Invariant:
- `pre_push_hook_blocks_push_when_policy_replay_fails`

Integration:
- `pre_push_policy_runs_dependency_checks_only_when_cargo_inputs_change`

Property-based (optional):
- not applicable

### Task 2: Define docs portability and range-safe docs checks

**Preconditions**

- Canonical docs lint and docs terminology scripts already exist.

**Invariants**

- Docs-only gates operate on markdown docs, not CSV task-registry files.
- Docs portability checks remain cheap enough for changed/range execution.

**Postconditions**

- The spec defines repo-portable docs references and range-safe markdown-only filtering.

**Tests (must exist before implementation)**

Unit:
- `doc_portability_rejects_machine_local_paths`
- `doc_portability_rejects_worktree_local_paths`

Invariant:
- `range_docs_filters_only_markdown_files`

Integration:
- `policy_checks_runs_doc_portability_in_range`

Property-based (optional):
- not applicable

### Task 3: Define path-sensitive flaky regression replay

**Preconditions**

- Required daemon invariant tests already exist and are runnable.

**Invariants**

- Replays are limited to known high-risk daemon regressions.
- Replays are skipped when no daemon-impacting files changed.

**Postconditions**

- The spec defines a bounded, repeated local replay gate for daemon regressions.

**Tests (must exist before implementation)**

Unit:
- `flaky_regressions_skip_unrelated_changes`

Invariant:
- `flaky_regressions_include_duplicate_submit_and_shutdown_cases`

Integration:
- `flaky_regressions_run_for_daemon_paths_with_configured_iterations`

Property-based (optional):
- not applicable

## Scenarios

- S1: A contributor adds a machine-local doc link and `pre-commit` rejects it before push.
- S2: A contributor changes CI/docs-range logic and the local pre-push replay catches markdown-scope mistakes before CI.
- S3: A dependency change introduces a new audit or deny failure and `pre-push` blocks the push locally.
- S4: A daemon concurrency regression reappears and repeated local replay catches it before `origin/main` sees the branch.

## Verification

- `scripts/install-bats.sh >/dev/null && "$(scripts/install-bats.sh)" scripts/tests/test-prepush-policy.bats`
- `scripts/doc-lint.sh --changed --strict-new`
- `scripts/check-tasks-registry.sh`
- `scripts/check-tasks-sync.sh --changed`
- `scripts/check-fast-feedback.sh`

## Risks and Failure Modes

- Pre-push replay runtime can become annoying if path-sensitive skips are not maintained carefully.
- Docs portability checks can produce false positives if contributors intentionally document machine-specific examples.
- Replaying known flaky tests locally can still miss failures if the set of unstable tests drifts over time.

## Open Questions

- Should future rollout expand pre-push replay to include fuzz smoke when `Cargo.toml` or daemon paths change?

## References

- [workflow-tool-guides.md](workflow-tool-guides.md)
- [deterministic-workflow-hardening.md](deterministic-workflow-hardening.md)
