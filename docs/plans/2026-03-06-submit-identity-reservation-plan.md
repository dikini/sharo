# Submit Identity Reservation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** durably reserve submit identities and idempotency ownership before reasoning starts so duplicate submits do not double-execute provider work and restart windows cannot reuse exposed IDs.

**Architecture:** move submit identity allocation from derived read-time hints to a durable reservation ledger in the store. Persist the high-water marks and in-flight idempotency ownership before reasoning starts, then finalize or release those reservations in the terminal submit paths so duplicate requests never race into provider execution.

**Tech Stack:** Rust 2024, daemon store unit tests, daemon scenario tests.

---

Template-Profile: tdd-strict-v1
Task-Registry-Refs: TASK-SUBMIT-IDENTITY-SPEC-001, TASK-SUBMIT-IDENTITY-PLAN-001

### Task 1: Persist submit reservations before reasoning

**Files:**
- Modify: `crates/sharo-daemon/src/store.rs`
- Modify: `crates/sharo-daemon/src/main.rs`
- Modify: `crates/sharo-daemon/tests/scenario_a.rs`
- Modify: `CHANGELOG.md`

**Preconditions**

- Existing concurrent submit tests pass.

**Invariants**

- Duplicate `session_id` + `idempotency_key` submits cannot both execute provider reasoning.
- Concurrent non-idempotent submits cannot reuse reserved task or turn identities.
- Restarting after reservation must not reuse the reserved task or turn IDs.

**Postconditions**

- Same-session concurrent preparations receive distinct task and turn identities.
- Duplicate in-flight idempotent submits get a deterministic non-executing replay outcome.
- Reopened stores allocate subsequent reservations from the persisted high-water marks.
- Reasoning input uses the durably reserved identities rather than derived read-time hints.
- Same-process retries after a terminal submit save failure can prepare the same idempotency key again.

**Tests (must exist before implementation)**

Unit:
- `prepare_submit_reserves_unique_hints_under_concurrency`
- `prepare_submit_blocks_duplicate_inflight_idempotency_keys`
- `reopened_store_keeps_reserved_identity_high_water_marks`
- `release_submit_reservation_clears_inflight_retry_after_commit_failure`

Property:
- `concurrent_same_session_submits_never_share_turn_or_task_hints`

Integration:
- `parallel_same_session_submits_produce_distinct_trace_scopes`
- `duplicate_submit_during_inflight_reasoning_does_not_double_execute_provider`

**Red Phase (required before code changes)**

Run: `cargo test -p sharo-daemon --bin sharo-daemon prepare_submit_blocks_duplicate_inflight_idempotency_keys -- --exact`
Expected: FAIL because the current preparation does not durably reserve in-flight idempotency ownership.

**Implementation Steps**

1. Add failing store coverage for duplicate in-flight idempotency and restart-safe reservation high-water marks.
2. Add scenario coverage that proves a duplicate in-flight submit does not execute provider work twice.
3. Extend persisted store state with durable reservation metadata for task IDs, turn IDs, and in-flight idempotency ownership.
4. Refactor `prepare_submit` to commit reservations before returning `Ready`, and return a non-executing replay outcome for duplicate in-flight idempotency keys.
5. Update terminal submit paths to finalize or release reservations consistently on success, fit-loop failure, connector/resolver failure, and final persist failure.
6. Recover stale in-flight idempotency reservations during `Store::open()` before serving new submits.
7. Re-run focused tests and the daemon crate suite.

**Green Phase (required)**

Run: `cargo test -p sharo-daemon --bin sharo-daemon prepare_submit_blocks_duplicate_inflight_idempotency_keys -- --exact`
Expected: PASS.

**Completion Evidence**

- Focused red/green test recorded
- Duplicate in-flight submit regression recorded
- Restart-high-water-mark regression recorded
- `cargo test -p sharo-daemon` passes
- `scripts/check-fast-feedback.sh` passes
- `CHANGELOG.md` updated
