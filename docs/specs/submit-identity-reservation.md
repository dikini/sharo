# Submit Identity Reservation

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
Task-Registry-Refs: TASK-SUBMIT-IDENTITY-SPEC-001, TASK-SUBMIT-IDENTITY-PLAN-001

## Purpose

Reserve submit identities and idempotency ownership durably before reasoning starts so concurrent duplicate submits never double-execute provider work and crash/restart windows never reuse externally visible task, trace, or turn identities.

## Scope

### In Scope

- durable submit preparation identity allocation
- concurrency-safe reservation of task, trace, and turn identifiers
- durable in-flight idempotency ownership for duplicate submit suppression
- restart-safe recovery behavior for uncommitted reservations
- regression tests for concurrent duplicate submits and restart-after-reservation windows

### Out of Scope

- protocol redesign beyond submit replay/error behavior required by reservation safety
- changing persisted task ID format
- external trace storage

## Core Terms

- Hint identity: pre-reasoning task, trace, and turn identifiers
- Reservation: a submit identity allocated and durably recorded before reasoning starts
- In-flight reservation ledger: persisted store state that records uncommitted reserved submits and their idempotency ownership
- Replay path: idempotent duplicate submit path that returns a committed result or an explicit reserved-submit response instead of re-running reasoning

## Interfaces / Contracts

- Concurrent non-idempotent submits must never share the same `task_id_hint`, `trace_id`, or `turn_id_hint`.
- Concurrent duplicate submits with the same `session_id` and `idempotency_key` must not execute provider reasoning more than once.
- Idempotent replay returns the committed task, committed failure, or an explicit in-flight reservation result that prevents duplicate execution while the original submit is unresolved.
- Reserved identities must be recorded durably before reasoning starts and must not be reused after daemon restart.
- Store reopen must recover abandoned in-flight idempotency reservations into deterministic replayable failures before new submit preparation resumes.

## Invariants

- Identity allocation is monotonic across both committed tasks and persisted reservations.
- Same-session concurrent submits observe distinct turn IDs.
- Persisted task IDs remain unique and monotonic.
- A persisted in-flight idempotency reservation blocks concurrent duplicate execution until the reservation is finalized.
- Store reopen explicitly recovers stale in-flight idempotency reservations into replayable failures instead of leaving them unresolved.

## Task Contracts

### Task 1: Reserve submit identity and idempotency ownership before reasoning

**Preconditions**

- Existing concurrent submit tests pass.

**Invariants**

- Duplicate `session_id` + `idempotency_key` submissions cannot both reach provider execution.
- Concurrent submit preparation cannot reuse a reserved identity.
- Restarted daemons must initialize future task/turn reservations from the durable high-water mark, not from committed tasks alone.

**Postconditions**

- Preparation durably records a uniquely reserved identity for each concurrent non-replay submit.
- Duplicate in-flight idempotent submits receive a deterministic non-executing replay outcome instead of `Ready`.
- Regression coverage proves same-session concurrent preparations diverge in both task and turn hints.
- Regression coverage proves a restart after reservation cannot reuse the previously exposed task or turn identity.
- Regression coverage proves reopening the store converts stale in-flight idempotency ownership into a replayable failure outcome.
- Regression coverage proves a terminal submit persist failure does not leave the idempotency key stuck in `submit_in_progress` for same-process retries.
- Regression coverage proves connector/resolver failure memoization save errors also release the same-process retry lock.

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
- `same_process_retry_after_failure_memoization_save_failure_is_not_stuck_in_progress`

## Verification

- `cargo test -p sharo-daemon prepare_submit_reserves_unique_hints_under_concurrency -- --exact`
- `cargo test -p sharo-daemon --bin sharo-daemon prepare_submit_blocks_duplicate_inflight_idempotency_keys -- --exact`
- `cargo test -p sharo-daemon --bin sharo-daemon reopened_store_keeps_reserved_identity_high_water_marks -- --exact`
- `cargo test -p sharo-daemon`

## Risks and Failure Modes

- Leaking durable reservations so duplicates remain blocked forever after terminal commit
- Leaking in-memory reservations after a terminal submit save failure
- Leaking in-memory reservations after connector or resolver failure memoization itself cannot be persisted
- Reusing preallocated turn IDs under contention or after restart
- Returning an in-flight replay signal that callers mis-handle as a terminal failure
- Diverging high-water marks between committed tasks and reservation ledger

## Open Questions

- None.

## References

- `crates/sharo-daemon/src/store.rs`
- `crates/sharo-daemon/src/kernel.rs`
