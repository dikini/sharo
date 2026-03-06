# Daemon Concurrent IPC Serving

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

Task-Registry-Refs: TASK-DAEMON-CONCURRENCY-SPEC-001, TASK-DAEMON-CONCURRENCY-PLAN-001

## Purpose

Ensure the daemon can accept and serve independent IPC requests concurrently so one slow task submission does not stall unrelated clients.

## Scope

### In Scope

- Concurrent connection handling in `sharo-daemon`.
- Safe separation of request serving from store mutation and blocking connector execution.
- Regression tests proving status and approval requests remain responsive while a submit is in flight.

### Out of Scope

- Multi-process sharding or distributed task execution.
- Protocol redesign for CLI or daemon envelopes.
- Replacing the persistent store format.

## Core Terms

- `Concurrent Request Serving`: accepting additional socket connections while earlier requests are still active.
- `Store Critical Section`: the minimum region requiring exclusive mutable access to persisted state.
- `Long-Running Submit`: a request path that waits on reasoning or provider I/O.

## Interfaces / Contracts

- Listener accept loop must not await full request handling inline.
- Exclusive store access must not be held across blocking provider work or `.await`.
- `submit`, `get-task`, `get-trace`, `get-artifacts`, and approval endpoints must continue to honor the existing IPC schema.

## Invariants

- One request still yields exactly one response.
- Socket framing and error envelopes remain unchanged.
- No mutable store guard is held across async suspension or provider execution.

## Task Contracts

### Task 1: Split Accept Loop From Request Execution

**Preconditions**

- Unix socket server is the daemon entrypoint.

**Invariants**

- Tokio request tasks own connection lifecycle independently.
- Shared runtime state is synchronized explicitly rather than by sequential accept-loop ordering.

**Postconditions**

- The daemon accepts new connections while earlier requests are still executing.

**Tests (must exist before implementation)**

Unit:
- `handle_request_avoids_holding_store_lock_across_provider_work`

Property:
- `serve_many_requests_returns_exactly_one_response_each`

Integration:
- `status_request_remains_responsive_during_slow_submit`
- `approval_list_remains_responsive_during_slow_submit`

## Scenarios

- S1: a slow submit runs against a blocking connector while `get-task` returns for a previously created task.
- S2: malformed IPC input on one connection does not stall acceptance of another connection.
- S3: `serve_once` remains deterministic for smoke usage.

## Verification

- `cargo test -p sharo-daemon --test daemon_ipc -- --nocapture`
- `cargo test -p sharo-daemon --test scenario_a status_request_remains_responsive_during_slow_submit -- --nocapture`
- `scripts/check-fast-feedback.sh`

## Risks and Failure Modes

- Overly broad locking could preserve the existing stall in a less obvious form.
- Moving request handling to spawned tasks can introduce shutdown-order bugs if state ownership is unclear.

## Open Questions

- Should the store use `Mutex` or `RwLock` once read-heavy endpoints are concurrent?

## References

- [docs/specs/ipc-transport.md](/home/dikini/Projects/sharo/docs/specs/ipc-transport.md)
- Rust skills: `async-no-lock-await`, `async-joinset-structured`, `own-arc-shared`
