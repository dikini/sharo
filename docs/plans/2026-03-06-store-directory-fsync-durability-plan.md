# Store Directory Fsync Durability Implementation Plan

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Property, and Integration tests.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: make successful store saves durable by syncing the parent directory after atomic rename.
Architecture: keep the existing temp-file write plus rename model, but add an explicit Unix directory sync step before reporting success. Verify that the helper still composes with the transactional store commit flow.
Tech Stack: Rust 2024, Unix filesystem sync, daemon store tests.
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-STORE-DIR-FSYNC-SPEC-001, TASK-STORE-DIR-FSYNC-PLAN-001

---

### Task 1: Add failing durability coverage and directory sync

**Files:**

- Modify: `crates/sharo-daemon/src/store.rs`

**Preconditions**

- Current transactional store tests pass.

**Invariants**

- Save still writes to a temp file and renames atomically.
- Store rollback semantics remain unchanged on failure.

**Postconditions**

- Successful save path syncs the parent directory before returning.

**Tests (must exist before implementation)**

Unit:
- `save_state_syncs_parent_directory_after_rename`

Property:
- `successful_save_state_always_performs_file_then_directory_sync_sequence`

Integration:
- `store_commit_path_retains_transactional_behavior_after_directory_sync`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-daemon store::tests -- --nocapture`
Expected: new durability coverage fails before directory sync is added.

**Implementation Steps**

1. Add a narrow helper for syncing the store parent directory on Unix.
2. Invoke it after `rename` and before success is returned.
3. Re-run transactional store tests to ensure rollback behavior is preserved.

**Green Phase (required)**

Command: `cargo test -p sharo-daemon store::tests -- --nocapture`
Expected: store tests pass with directory sync included.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/sharo-daemon/src/store.rs`
Re-run: `cargo test -p sharo-daemon`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated
