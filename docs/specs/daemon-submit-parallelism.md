# Daemon Submit Parallelism

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
Task-Registry-Refs: TASK-DAEMON-SUBMIT-PARALLELISM-SPEC-001, TASK-DAEMON-SUBMIT-PARALLELISM-PLAN-001

## Purpose

Remove process-wide serialization of submit requests so independent provider-backed submits can run concurrently.

## Scope

### In Scope

- daemon submit request concurrency
- submit-scoped idempotency and store critical sections
- regression tests proving concurrent submit progress

### Out of Scope

- backward-compatibility shims
- protocol changes
- connector retry policy

## Core Terms

- Submit guard: the synchronization around `SubmitTask`
- Preparation: pre-reasoning idempotency and hint calculation
- Commit phase: final store mutation after reasoning completes

## Interfaces / Contracts

- `SubmitTask` requests for different sessions or idempotency keys may run concurrently.
- Store access remains lock-bounded around preparation and commit only.
- Duplicate logical submits must still replay correctly without ghost tasks.

## Invariants

- No mutex is held across provider-backed reasoning.
- Idempotent replay semantics remain correct.
- Non-submit requests remain responsive during slow submits.

## Task Contracts

### Task 1: Remove global submit serialization

**Preconditions**

- Current daemon tests pass.

**Invariants**

- Store critical sections stay explicit and short.
- Concurrent submits cannot create duplicate committed tasks for the same idempotency key.

**Postconditions**

- Two independent slow submits can make provider progress concurrently.

**Tests (must exist before implementation)**

Unit:
- `submit_requests_do_not_share_process_wide_guard`

Property:
- `independent_submit_requests_can_progress_in_parallel`

Integration:
- `concurrent_slow_submits_make_parallel_upstream_progress`

## Scenarios

- Two slow submits from different sessions overlap instead of serializing.
- A duplicate idempotent retry still replays the first committed result.

## Verification

- `cargo test -p sharo-daemon --test scenario_a concurrent_slow_submits_make_parallel_upstream_progress -- --nocapture`
- `cargo test -p sharo-daemon`

## Risks and Failure Modes

- Reintroducing long-lived store locks
- Breaking duplicate-submit replay semantics
- Hidden shared mutable state in submit execution

## Open Questions

- None.

## References

- `crates/sharo-daemon/src/main.rs`
