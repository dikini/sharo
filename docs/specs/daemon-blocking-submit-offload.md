# Daemon Blocking Submit Offload

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
Task-Registry-Refs: TASK-DAEMON-BLOCKING-OFFLOAD-SPEC-001, TASK-DAEMON-BLOCKING-OFFLOAD-PLAN-001

## Purpose

Prevent provider-backed submit processing from blocking Tokio runtime worker threads.

## Scope

### In Scope

- daemon request execution model for blocking submit work
- runtime responsiveness under concurrent slow submits
- regression tests that prove non-submit IPC requests continue to make progress while runtime workers are saturated

### Out of Scope

- protocol changes
- connector retry policy
- changing store persistence semantics

## Core Terms

- Runtime worker: a Tokio scheduler thread serving daemon IPC tasks
- Blocking submit work: synchronous reasoning, connector execution, and store fsync work triggered by `SubmitTask`
- Offload boundary: the handoff from async IPC handling to dedicated blocking execution

## Interfaces / Contracts

- Async IPC handling must not run provider-backed submit execution directly on Tokio runtime worker threads.
- Blocking submit work must run behind an explicit blocking executor boundary.
- Non-submit IPC requests must remain serviceable while multiple slow submits are in flight.

## Invariants

- No Tokio runtime worker should be monopolized by synchronous submit execution.
- Store locking semantics remain unchanged or narrower after offload.
- Existing submit replay and task persistence semantics remain correct.

## Task Contracts

### Task 1: Offload blocking submit execution

**Preconditions**

- Existing daemon IPC and submit scenarios pass.

**Invariants**

- Submit execution still produces the same persisted task, trace, and artifact outputs.
- Slow submits do not prevent independent request handling.

**Postconditions**

- Provider-backed submit execution runs behind a blocking executor boundary.
- Regression coverage proves daemon responsiveness under concurrent slow submits.

**Tests (must exist before implementation)**

Unit:
- `submit_execution_runs_outside_runtime_worker`

Property:
- `runtime_workers_remain_available_under_slow_submit_pressure`

Integration:
- `status_requests_remain_responsive_under_parallel_slow_submits`

## Scenarios

- Several slow submits are started concurrently while a status request still returns promptly.
- A slow submit that fails still records idempotent failure state without blocking unrelated requests.

## Verification

- `cargo test -p sharo-daemon --test daemon_ipc status_requests_remain_responsive_under_parallel_slow_submits -- --nocapture`
- `cargo test -p sharo-daemon`

## Risks and Failure Modes

- Moving work incorrectly and holding a mutex across a blocking boundary
- Spawning detached work that can outlive request response ownership
- Regressing idempotent failure recording

## Open Questions

- None.

## References

- `crates/sharo-daemon/src/main.rs`
- `crates/sharo-daemon/tests/daemon_ipc.rs`
