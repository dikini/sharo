# Deterministic Workflow Hardening Implementation Plan

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Invariant, and Integration checks.
> - Use Property-based tests only when a generative framework is actually used.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: harden merge and runtime verification workflow so high-risk regressions are deterministically blocked before changes are finalized on `main`.
Architecture: add narrow verification scripts for merge compatibility, conflict determinism, runtime invariants, and durability signals; wire them into fast-feedback and policy paths incrementally. Keep each slice test-first with script-level Bats coverage and targeted Rust test reuse rather than large framework changes.
Tech Stack: Rust 2024 workspace tests, Bash gate scripts, Bats script tests, existing `scripts/check-fast-feedback.sh` and policy checks.
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-DETERMINISTIC-WORKFLOW-PLAN-001, TASK-DETERMINISTIC-WORKFLOW-SPEC-001

---

### Task 1: Add merge-result compatibility gate

**Files:**

- Create: `scripts/check-merge-compat.sh`
- Create: `scripts/tests/test-merge-compat.bats`
- Modify: `scripts/check-fast-feedback.sh`
- Modify: `.githooks/pre-commit`

**Preconditions**

- Existing workspace checks are passing from `scripts/check-fast-feedback.sh`.

**Invariants**

- Merge-result gate runs on current tree only.
- Gate remains deterministic and non-interactive.

**Postconditions**

- Merge-result compatibility drift is detected by one script invocation.

**Tests (must exist before implementation)**

Unit:
- `test_merge_compat_fails_when_required_protocol_field_missing`

Invariant:
- `test_merge_compat_uses_current_tree_not_parent_heads`

Integration:
- `test_merge_compat_runs_clippy_and_workspace_tests`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `bats scripts/tests/test-merge-compat.bats`
Expected: fails because script and assertions do not yet exist.

**Implementation Steps**

1. Add `scripts/tests/test-merge-compat.bats` with failing expectations for missing script behavior.
2. Implement `scripts/check-merge-compat.sh` to run `cargo clippy --all-targets --all-features -- -D warnings` and `cargo test --workspace`.
3. Wire the script into `scripts/check-fast-feedback.sh` and pre-commit flow with changed-file scoping where possible.

**Green Phase (required)**

Command: `bats scripts/tests/test-merge-compat.bats`
Expected: all merge-compat script tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `scripts/check-merge-compat.sh`, `scripts/tests/test-merge-compat.bats`
Re-run: `scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 2: Add deterministic conflict-policy gate for high-churn files

**Files:**

- Create: `docs/specs/conflict-resolution-policy.md`
- Create: `scripts/check-conflict-determinism.sh`
- Create: `scripts/tests/test-conflict-determinism.bats`
- Modify: `scripts/check-fast-feedback.sh`

**Preconditions**

- Merge-result gate from Task 1 exists.

**Invariants**

- High-churn file policy is explicit and script-validated.
- Gate fails on unresolved conflict markers and policy mismatches.

**Postconditions**

- Conflict outcomes for designated files are deterministic and machine-verifiable.

**Tests (must exist before implementation)**

Unit:
- `test_conflict_policy_detects_unresolved_markers`

Invariant:
- `test_conflict_policy_enforces_known_file_rules`

Integration:
- `test_conflict_policy_runs_in_fast_feedback_path`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `bats scripts/tests/test-conflict-determinism.bats`
Expected: fails because policy script and policy doc do not yet exist.

**Implementation Steps**

1. Add failing Bats tests for unresolved marker and allowlist enforcement.
2. Define deterministic conflict policy in `docs/specs/conflict-resolution-policy.md`.
3. Implement `scripts/check-conflict-determinism.sh` to validate policy and blocked patterns.
4. Wire gate into `scripts/check-fast-feedback.sh`.

**Green Phase (required)**

Command: `bats scripts/tests/test-conflict-determinism.bats`
Expected: policy gate tests pass and failures are deterministic.

**Refactor Phase (optional but controlled)**

Allowed scope: conflict policy doc and script files only
Re-run: `scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 3: Add runtime invariant script gate for idempotency/concurrency/shutdown

**Files:**

- Create: `scripts/check-daemon-invariants.sh`
- Create: `scripts/tests/test-daemon-invariants.bats`
- Modify: `scripts/check-fast-feedback.sh`
- Modify: `.githooks/pre-commit`

**Preconditions**

- Existing daemon scenario tests pass from baseline.

**Invariants**

- Script runs only targeted invariant tests and remains deterministic.
- Coverage includes duplicate submit, retry unlock, responsiveness, and ctrl-c drain.

**Postconditions**

- One script invocation validates the highest-risk daemon semantics.

**Tests (must exist before implementation)**

Unit:
- `test_daemon_invariants_script_includes_required_cases`

Invariant:
- `duplicate_submit_during_inflight_reasoning_does_not_double_execute_provider`
- `same_process_retry_after_terminal_save_failure_is_not_stuck_in_progress`

Integration:
- `status_requests_remain_responsive_under_parallel_slow_submits`
- `ctrl_c_waits_for_inflight_request_completion`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `bats scripts/tests/test-daemon-invariants.bats`
Expected: fails because runtime invariant script does not yet exist.

**Implementation Steps**

1. Add failing script tests that require exact named scenario and daemon IPC cases.
2. Implement `scripts/check-daemon-invariants.sh` to run those named tests with explicit commands.
3. Add script to fast-feedback and pre-commit policy path for Rust-impacting changes.

**Green Phase (required)**

Command: `bats scripts/tests/test-daemon-invariants.bats && scripts/check-daemon-invariants.sh`
Expected: script tests pass and runtime invariants pass.

**Refactor Phase (optional but controlled)**

Allowed scope: daemon invariant script and its Bats test only
Re-run: `scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 4: Add durability-signal visibility gate

**Files:**

- Create: `scripts/check-durability-signals.sh`
- Create: `scripts/tests/test-durability-signals.bats`
- Modify: `scripts/check-fast-feedback.sh`

**Preconditions**

- Store warning-path tests exist in daemon unit suite.

**Invariants**

- Gate fails if degraded-durability warning signal checks are removed or renamed without policy update.

**Postconditions**

- Durability warning semantics are continuously asserted via script gate.

**Tests (must exist before implementation)**

Unit:
- `test_durability_signal_script_checks_warning_assertions`

Invariant:
- `post_rename_directory_sync_failure_emits_warning_signal`

Integration:
- `test_fast_feedback_invokes_durability_signal_gate`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `bats scripts/tests/test-durability-signals.bats`
Expected: fails because script is absent.

**Implementation Steps**

1. Add failing Bats tests that assert exact daemon store warning test coverage in the script.
2. Implement `scripts/check-durability-signals.sh` with explicit targeted test command(s).
3. Wire script into fast-feedback.

**Green Phase (required)**

Command: `bats scripts/tests/test-durability-signals.bats && scripts/check-durability-signals.sh`
Expected: script tests and durability signal checks pass.

**Refactor Phase (optional but controlled)**

Allowed scope: durability gate script and test files only
Re-run: `scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 5: Optional merged-worktree cleanup automation

**Files:**

- Create: `scripts/cleanup-merged-worktrees.sh`
- Create: `scripts/tests/test-cleanup-merged-worktrees.bats`
- Modify: `docs/tasks/README.md`

**Preconditions**

- Clear policy exists for force handling and protected branches.

**Invariants**

- Cleanup script never deletes unmerged branches.
- Dirty worktrees require explicit `--force`.

**Postconditions**

- Operators have deterministic cleanup tooling after local merges.

**Tests (must exist before implementation)**

Unit:
- `test_cleanup_script_skips_unmerged_branches`

Invariant:
- `test_cleanup_script_requires_explicit_force_for_dirty_worktree`

Integration:
- `test_cleanup_script_removes_merged_worktree_and_branch`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `bats scripts/tests/test-cleanup-merged-worktrees.bats`
Expected: fails while cleanup script is missing.

**Implementation Steps**

1. Add failing Bats cases for unmerged and dirty-worktree protections.
2. Implement `scripts/cleanup-merged-worktrees.sh` with dry-run default and `--apply`.
3. Document safe invocation and guardrails in `docs/tasks/README.md`.

**Green Phase (required)**

Command: `bats scripts/tests/test-cleanup-merged-worktrees.bats`
Expected: all cleanup guardrail tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: cleanup script and docs only
Re-run: `scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated
