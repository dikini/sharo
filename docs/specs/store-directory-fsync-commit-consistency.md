# Store Directory Fsync Commit Consistency

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Define tests before implementation work.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Property, and Integration tests.
> - Run `scripts/doc-lint.sh` before commit.

Updated: 2026-03-06
Status: active
Owner: codex
Template-Profile: tdd-strict-v1
Task-Registry-Refs: TASK-STORE-FSYNC-CONSISTENCY-SPEC-001, TASK-STORE-FSYNC-CONSISTENCY-PLAN-001

## Purpose

Keep in-memory and on-disk store state consistent when the parent-directory fsync step fails.

## Scope

### In Scope

- transactional store commit semantics around rename and directory fsync
- deterministic unit tests for post-rename failure behavior
- explicit failure handling after the filesystem already contains the new state

### Out of Scope

- multi-process store coordination
- changing the JSON persistence format
- non-Unix portability work

## Core Terms

- Commit state: the `PersistedState` that callers observe after a successful mutation
- Post-rename durability step: syncing the parent directory after the new file is renamed into place
- Consistency contract: in-memory state and canonical on-disk state must not diverge after a reported outcome

## Interfaces / Contracts

- If the canonical store file has already been replaced with new state, the in-memory store must converge to the same state even if directory fsync fails.
- Post-rename directory-fsync failure is a degraded-durability warning, not a failed logical mutation.
- Save-path tests must distinguish pre-rename write failures from post-rename durability failures.

## Invariants

- No mutation may leave memory behind disk after return.
- Pre-rename failures still roll back fully.
- Persisted permissions remain restricted.

## Task Contracts

### Task 1: Preserve memory/disk consistency on post-rename fsync failure

**Preconditions**

- Existing store rollback tests pass.

**Invariants**

- Pre-rename save failures still return without mutating `self.state`.
- Post-rename failures cannot reintroduce ghost stale memory.
- Post-rename durability warnings must not cause callers to retry non-idempotent operations as though nothing committed.

**Postconditions**

- A simulated directory-fsync failure after rename leaves in-memory state matching the new on-disk file.
- The mutation still returns success to the caller after rename, so non-idempotent retries are not induced by a false failure signal.

**Tests (must exist before implementation)**

Unit:
- `post_rename_directory_sync_failure_keeps_memory_and_disk_consistent`
- `post_rename_directory_sync_failure_returns_committed_result`

Property:
- `commit_outcome_never_leaves_memory_behind_disk`

Integration:
- `idempotent_retry_after_post_rename_sync_failure_replays_committed_task`

## Verification

- `cargo test -p sharo-daemon post_rename_directory_sync_failure_keeps_memory_and_disk_consistent -- --exact`
- `cargo test -p sharo-daemon`

## Risks and Failure Modes

- Treating a logically committed mutation as rolled back
- Inducing duplicate non-idempotent retries from a false failed-mutation signal
- Regressing existing rollback semantics for pre-rename failures

## Open Questions

- None.

## References

- `crates/sharo-daemon/src/store.rs`
