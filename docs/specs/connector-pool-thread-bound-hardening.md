# Connector Pool Thread Bound Hardening

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Define tests before implementation work.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Property, and Integration tests.
> - Run `scripts/doc-lint.sh` before commit.

Updated: 2026-03-06
Status: active
Owner: runtime
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-CONNECTOR-POOL-HARDENING-SPEC-001, TASK-CONNECTOR-POOL-HARDENING-PLAN-001

## Purpose

Close the race in daemon connector-pool scale-up so configured worker bounds are enforced even under concurrent submit pressure.

## Scope

### In Scope

- Hardening `BlockingPool` scale-up and worker-accounting logic.
- Making the max-worker bound mechanically enforced instead of advisory.
- Adding race-oriented tests that validate the bound under concurrent submission.

### Out of Scope

- Replacing the pool with a new executor model.
- Changing provider protocol behavior.
- Adding new user-facing configuration fields.

## Core Terms

- `ScaleUpPermit`: a synchronization step that grants the right to create one new worker.
- `WorkerBound`: the invariant that active workers never exceed `max_threads`.
- `Concurrent Submit Burst`: many `submit()` calls racing on the same pool instance.

## Interfaces / Contracts

- `BlockingPool::submit` may trigger scale-up, but only through an atomic permit path.
- `BlockingPool::current_worker_count` must remain within `[min_threads, max_threads]` for the lifetime of the pool.
- Pool saturation must still surface `PoolError::Overloaded` deterministically.

## Invariants

- `active_workers <= max_threads` must hold under all interleavings.
- Failed or panicking jobs must not corrupt worker accounting.
- Existing fixed-size and overload behavior must remain unchanged.

## Task Contracts

### Task 1: Make Scale-Up Bound Atomic

**Preconditions**

- `BlockingPool` remains the daemon execution surface for blocking connectors.

**Invariants**

- Scale-up uses one reservation path per worker creation.
- No code path increments worker count after the max bound has already been reached.

**Postconditions**

- Concurrent submitters cannot cause worker count to exceed `max_threads`.

**Tests (must exist before implementation)**

Unit:
- `scale_up_reservation_is_atomic_under_race`

Property:
- `worker_count_never_exceeds_max_threads_under_parallel_submit`

Integration:
- `daemon_burst_submit_never_exceeds_configured_connector_workers`

## Scenarios

- S1: two simultaneous submissions race when `active_workers == max_threads - 1`; only one worker is created.
- S2: queue pressure reaches the scale-up threshold repeatedly; worker count caps at `max_threads`.
- S3: a panicking job does not leak worker permits or disable later scale decisions.

## Verification

- `cargo test -p sharo-daemon connector_pool::tests::scale_up_reservation_is_atomic_under_race -- --nocapture`
- `cargo test -p sharo-daemon connector_pool::tests::worker_count_never_exceeds_max_threads_under_parallel_submit -- --nocapture`
- `cargo test -p sharo-daemon --test scenario_a -- --nocapture`
- `scripts/check-fast-feedback.sh`

## Risks and Failure Modes

- Over-serialization of scale-up could reduce responsiveness if the reservation path is too coarse.
- Incorrect rollback on failed worker spawn could undercount workers and trigger churn.

## Open Questions

- Should worker creation expose debug telemetry for reserved versus active counts?

## References

- [docs/specs/connector-blocking-execution.md](/home/dikini/Projects/sharo/docs/specs/connector-blocking-execution.md)
- Rust skills: `async-bounded-channel`, `own-arc-shared`, `test-proptest-properties`
