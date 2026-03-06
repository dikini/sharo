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

Reserve submit identities atomically so concurrent submits never reuse the same task, trace, or turn hints.

## Scope

### In Scope

- submit preparation identity allocation
- concurrency-safe reservation of task and turn identifiers
- regression tests for same-session concurrent submit preparation

### Out of Scope

- protocol redesign
- changing persisted task ID format
- external trace storage

## Core Terms

- Hint identity: pre-reasoning task, trace, and turn identifiers
- Reservation: an identity allocated before reasoning starts and unavailable to concurrent submit preparations
- Replay path: idempotent duplicate submit path that returns an existing result instead of reserving a new identity

## Interfaces / Contracts

- Concurrent non-idempotent submits must never share the same `task_id_hint`, `trace_id`, or `turn_id_hint`.
- Idempotent replay still returns the original committed task or stored failure.
- Reserved identities must either commit or be safely discarded without colliding with future reservations.

## Invariants

- Identity allocation is monotonic.
- Same-session concurrent submits observe distinct turn IDs.
- Persisted task IDs remain unique and monotonic.

## Task Contracts

### Task 1: Reserve submit identity before reasoning

**Preconditions**

- Existing concurrent submit tests pass.

**Invariants**

- Replay semantics remain unchanged for duplicate idempotency keys.
- Concurrent submit preparation cannot reuse a reserved identity.

**Postconditions**

- Preparation returns a uniquely reserved identity for each concurrent non-replay submit.
- Regression coverage proves same-session concurrent preparations diverge in both task and turn hints.

**Tests (must exist before implementation)**

Unit:
- `prepare_submit_reserves_unique_hints_under_concurrency`

Property:
- `concurrent_same_session_submits_never_share_turn_or_task_hints`

Integration:
- `parallel_same_session_submits_produce_distinct_trace_scopes`

## Verification

- `cargo test -p sharo-daemon prepare_submit_reserves_unique_hints_under_concurrency -- --exact`
- `cargo test -p sharo-daemon`

## Risks and Failure Modes

- Leaking reserved identities into replay paths
- Reusing preallocated turn IDs under contention
- Coupling reservation too tightly to commit ordering

## Open Questions

- None.

## References

- `crates/sharo-daemon/src/store.rs`
- `crates/sharo-daemon/src/kernel.rs`
