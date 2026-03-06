# Store Directory Fsync Durability

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
Task-Registry-Refs: TASK-STORE-DIR-FSYNC-SPEC-001, TASK-STORE-DIR-FSYNC-PLAN-001

## Purpose

Make successful store commits durable across crash boundaries by syncing the parent directory after atomic rename.

## Scope

### In Scope

- directory fsync after store rename
- narrow persistence helper changes
- regression coverage for the durability sequence

### Out of Scope

- cross-platform non-Unix persistence redesign
- write batching

## Interfaces / Contracts

- A successful store save must sync file contents and the parent directory entry update.

## Invariants

- Store writes remain temp-file-plus-rename based.
- File mode restrictions stay `0o600`.

## Task Contracts

### Task 1: Add directory durability step

**Preconditions**

- Current store tests pass.

**Invariants**

- Transactional commit behavior stays unchanged.
- Save failure still leaves in-memory state untouched.

**Postconditions**

- The parent directory is synced after rename and before success is reported.

**Tests (must exist before implementation)**

Unit:
- `save_state_syncs_parent_directory_after_rename`

Property:
- `successful_save_state_always_performs_file_then_directory_sync_sequence`

Integration:
- `store_commit_path_retains_transactional_behavior_after_directory_sync`

## Verification

- `cargo test -p sharo-daemon store::tests -- --nocapture`
- `cargo test -p sharo-daemon`

## Risks and Failure Modes

- Crash window after rename but before directory metadata is persisted
- Regressing the transactional store behavior

## References

- `crates/sharo-daemon/src/store.rs`
