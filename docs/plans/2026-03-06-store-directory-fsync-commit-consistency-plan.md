# Store Directory Fsync Commit Consistency Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** ensure the store never returns with stale in-memory state or false failed-mutation semantics after the canonical on-disk file has already been replaced, while still surfacing degraded durability to operators.

**Architecture:** split the save path into pre-rename persistence and post-rename durability phases. Model post-rename directory-fsync failure explicitly so `commit_mutation` can converge memory to the committed on-disk state, return the committed mutation result, and emit a warning signal carrying the degraded-durability message.

**Tech Stack:** Rust 2024, Unix filesystem APIs, daemon store unit tests.

---

Template-Profile: tdd-strict-v1
Task-Registry-Refs: TASK-STORE-FSYNC-CONSISTENCY-SPEC-001, TASK-STORE-FSYNC-CONSISTENCY-PLAN-001

### Task 1: Preserve memory and disk consistency on post-rename fsync failure

**Files:**
- Modify: `crates/sharo-daemon/src/store.rs`

**Preconditions**

- Existing store rollback tests pass.

**Invariants**

- Pre-rename save failures still leave `self.state` unchanged.
- Post-rename failures cannot leave memory behind disk.
- Post-rename durability warnings must not be surfaced as failed logical mutations.
- Post-rename durability warnings must still be observable outside the store.

**Postconditions**

- Post-rename directory-fsync failure keeps in-memory state aligned with the new on-disk file.
- The caller still receives the committed logical result after rename.
- The degraded-durability message is emitted through a warning path.

**Tests (must exist before implementation)**

Unit:
- `post_rename_directory_sync_failure_keeps_memory_and_disk_consistent`
- `post_rename_directory_sync_failure_returns_committed_result`
- `post_rename_directory_sync_failure_emits_warning_signal`

Property:
- `commit_outcome_never_leaves_memory_behind_disk`

Integration:
- `idempotent_retry_after_post_rename_sync_failure_replays_committed_task`

**Red Phase (required before code changes)**

Run: `cargo test -p sharo-daemon post_rename_directory_sync_failure_keeps_memory_and_disk_consistent -- --exact`
Expected: FAIL because current logic drops the durability warning signal after returning committed success.

**Implementation Steps**

1. Add a failing store unit test that simulates directory-fsync failure after rename and asserts the warning path is exercised.
2. Refactor the save path to preserve the degraded-durability message through the post-rename outcome.
3. Update `commit_mutation` so logically committed state is reflected in memory and returned to the caller even when durability reporting fails after rename.
4. Emit the degraded-durability warning via a dedicated helper that can be verified in tests and routed to operator-visible output in production.
5. Re-run focused store tests and the daemon crate suite.

**Green Phase (required)**

Run: `cargo test -p sharo-daemon post_rename_directory_sync_failure_keeps_memory_and_disk_consistent -- --exact`
Expected: PASS.

**Completion Evidence**

- Focused red/green test recorded
- `cargo test -p sharo-daemon` passes
- `scripts/check-fast-feedback.sh` passes
- `CHANGELOG.md` updated
