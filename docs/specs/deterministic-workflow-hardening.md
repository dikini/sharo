# Deterministic Workflow Hardening

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

Task-Registry-Refs: TASK-DETERMINISTIC-WORKFLOW-SPEC-001, TASK-DETERMINISTIC-WORKFLOW-PLAN-001

## Purpose

Define deterministic guardrails for long-running branch workflows so merge outcomes, retry and concurrency invariants, and degraded-durability signals are validated in a repeatable machine-enforced way before integration to `main`.

## Scope

### In Scope

- Merge-result compatibility verification on the exact post-merge tree.
- Deterministic conflict handling for high-churn artifacts (`CHANGELOG.md`, task registry, protocol shape tests).
- Required invariant checks for concurrency and idempotency, plus shutdown-drain behavior in daemon/runtime paths.
- Operator-visible degraded-durability signaling checks.
- Optional worktree lifecycle cleanup automation after merge.

### Out of Scope

- Changing persisted runtime data schema.
- Replacing git workflows with hosted PR automation.
- Introducing backward-compatibility constraints for pre-1.0 state files.

## Core Terms

- `Merge Result Gate`: checks executed on the exact merged tree, not per-parent branch.
- `Conflict Determinism`: scripted, repeatable conflict resolution policy for known files.
- `Runtime Invariant Suite`: tests covering duplicate submit, restart recovery, shutdown in-flight drain, and idempotent retry semantics.
- `Durability Degradation Signal`: observable warning path when post-rename directory fsync fails.

## Interfaces / Contracts

- `scripts/check-fast-feedback.sh` remains the mandatory local quality gate.
- Add `scripts/check-merge-compat.sh`:
  - runs compile/lint/tests on the merge-result working tree
  - fails on protocol-shape mismatch in tests
- Add `scripts/check-conflict-determinism.sh`:
  - validates known conflict-prone files are resolved according to policy
- Add `scripts/check-daemon-invariants.sh`:
  - runs targeted daemon scenarios for concurrency, idempotency, and shutdown
- Add `scripts/check-durability-signals.sh`:
  - verifies degraded-durability warnings are surfaced, not silently dropped
- Add optional `scripts/cleanup-merged-worktrees.sh`:
  - removes merged feature worktrees and branches unless explicitly preserved

## Invariants

- `main` remains canonical; merge success claims require fresh merge-result evidence.
- Retry safety: no same-process retry can be permanently blocked by stale in-memory reservations.
- Duplicate in-flight submit with the same idempotency key must not produce double provider execution.
- Ctrl-C shutdown drains accepted handlers to exactly one response per accepted connection.
- Post-rename fsync degradation remains visible through explicit warning signal checks.

## Task Contracts

### Task 1: Merge Result Compatibility Gate

**Preconditions**

- `main` and feature branch heads are known and mergeable.

**Invariants**

- Merge validations run on the merged worktree state only.

**Postconditions**

- `scripts/check-merge-compat.sh` fails deterministically on protocol and test-shape drift.

**Tests (must exist before implementation)**

Unit:
- `test_merge_compat_fails_when_required_protocol_field_missing`

Invariant:
- `test_merge_compat_uses_current_tree_not_parent_heads`

Integration:
- `test_merge_compat_runs_clippy_and_workspace_tests`

Property-based (optional):
- not applicable

### Task 2: Deterministic Conflict Policy Gate

**Preconditions**

- Conflict policy document and file allowlist are defined.

**Invariants**

- High-churn file conflict outcomes are machine-checked.

**Postconditions**

- `scripts/check-conflict-determinism.sh` rejects non-policy conflict artifacts.

**Tests (must exist before implementation)**

Unit:
- `test_conflict_policy_detects_unresolved_markers`

Invariant:
- `test_conflict_policy_enforces_known_file_rules`

Integration:
- `test_conflict_policy_runs_in_pre_commit_and_ci_path`

Property-based (optional):
- not applicable

### Task 3: Runtime Invariant Guardrail Suite

**Preconditions**

- Existing daemon scenario tests are stable.

**Invariants**

- Coverage includes idempotency, restart, concurrency responsiveness, and shutdown drain.

**Postconditions**

- `scripts/check-daemon-invariants.sh` provides a deterministic narrow gate for high-risk runtime semantics.

**Tests (must exist before implementation)**

Unit:
- `test_daemon_invariants_script_includes_required_cases`

Invariant:
- `test_duplicate_submit_during_inflight_reasoning_does_not_double_execute_provider`
- `test_same_process_retry_after_terminal_save_failure_is_not_stuck_in_progress`

Integration:
- `test_status_requests_remain_responsive_under_parallel_slow_submits`
- `test_ctrl_c_waits_for_inflight_request_completion`

Property-based (optional):
- not applicable

### Task 4: Durability Signal Visibility Gate

**Preconditions**

- Store durability warning path exists.

**Invariants**

- Committed-but-weak-durability outcomes are detectable by operators.

**Postconditions**

- `scripts/check-durability-signals.sh` fails when warning signal assertions are removed.

**Tests (must exist before implementation)**

Unit:
- `test_durability_signal_script_checks_warning_assertions`

Invariant:
- `test_post_rename_directory_sync_failure_emits_warning_signal`

Integration:
- `test_fast_feedback_invokes_durability_signal_gate`

Property-based (optional):
- not applicable

### Task 5: Branch Lifecycle Cleanup Automation (Optional Safety Slice)

**Preconditions**

- Branch and worktree merge policy is defined.

**Invariants**

- Cleanup never runs on unmerged branches without explicit force.

**Postconditions**

- `scripts/cleanup-merged-worktrees.sh` deterministically reports or removes only merged branches/worktrees.

**Tests (must exist before implementation)**

Unit:
- `test_cleanup_script_skips_unmerged_branches`

Invariant:
- `test_cleanup_script_requires_explicit_force_for_dirty_worktree`

Integration:
- `test_cleanup_script_removes_merged_worktree_and_branch`

Property-based (optional):
- not applicable

## Scenarios

- S1: feature branch merges cleanly but fails merge-result gate due to protocol field drift in tests.
- S2: conflict in `CHANGELOG.md` resolves non-deterministically and policy gate blocks commit.
- S3: runtime concurrency regression reappears and invariant suite fails before merge.
- S4: degraded durability warning path is accidentally dropped and durability gate fails.
- S5: merged-branch cleanup script encounters dirty worktree and refuses unsafe delete without force.

## Verification

- `scripts/doc-lint.sh --changed --strict-new`
- `scripts/check-fast-feedback.sh`
- `scripts/check-tasks-registry.sh`
- `scripts/check-tasks-sync.sh --changed`

## Risks and Failure Modes

- Over-constraining conflict policy can slow legitimate edits in high-churn files.
- New gates can increase local cycle time if not scoped to changed files.
- Cleanup automation can be unsafe if merge ancestry checks are incorrect.

## Open Questions

- Should merge-result gate be required only for merges into `main`, or for all local merges?
- Should cleanup automation stay opt-in (`--apply`) or default-on after local merge workflows?

## References

- [store-directory-fsync-commit-consistency.md](/home/dikini/Projects/sharo/docs/specs/store-directory-fsync-commit-consistency.md)
- [submit-identity-reservation.md](/home/dikini/Projects/sharo/docs/specs/submit-identity-reservation.md)
- [daemon-concurrent-ipc-serving.md](/home/dikini/Projects/sharo/docs/specs/daemon-concurrent-ipc-serving.md)
- Rust skills: `err-result-over-panic`, `async-no-lock-await`, `test-descriptive-names`, `lint-rustfmt-check`
