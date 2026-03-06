# Daemon Concurrent IPC Serving Implementation Plan

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Property, and Integration tests.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: let the daemon continue accepting and serving independent IPC requests while slow submissions are still running.
Architecture: move connection handling into spawned async tasks while reducing store access to explicit critical sections guarded by shared state. The design follows `rust-skills` async guidance: never hold exclusive state across `.await`, and keep blocking work off the request-serving path.
Tech Stack: Rust 2024, Tokio Unix sockets, `Arc`, synchronization primitives, existing daemon IPC tests.
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-DAEMON-CONCURRENCY-PLAN-001, TASK-DAEMON-CONCURRENCY-SPEC-001

---

### Task 1: Add failing responsiveness coverage

**Files:**

- Modify: `crates/sharo-daemon/tests/daemon_ipc.rs`
- Modify: `crates/sharo-daemon/tests/scenario_a.rs`

**Preconditions**

- Existing IPC roundtrip tests pass.

**Invariants**

- Response schema remains unchanged.
- New tests isolate responsiveness, not general throughput.

**Postconditions**

- There are deterministic tests proving a slow submit currently blocks unrelated requests.

**Tests (must exist before implementation)**

Unit:
- `handle_request_avoids_holding_store_lock_across_provider_work`

Property:
- `serve_many_requests_returns_exactly_one_response_each`

Integration:
- `status_request_remains_responsive_during_slow_submit`
- `approval_list_remains_responsive_during_slow_submit`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-daemon --test daemon_ipc --test scenario_a status_request_remains_responsive_during_slow_submit approval_list_remains_responsive_during_slow_submit -- --nocapture`
Expected: the new responsiveness tests fail against the current sequential request loop.

**Implementation Steps**

1. Add a slow-submit harness using an intentionally delayed connector path.
2. Add a second client request that must complete before the slow submit finishes.
3. Keep all assertions on observable daemon responses and timing bounds.

**Green Phase (required)**

Command: `cargo test -p sharo-daemon --test daemon_ipc --test scenario_a status_request_remains_responsive_during_slow_submit approval_list_remains_responsive_during_slow_submit -- --nocapture`
Expected: the new tests pass after concurrency changes.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/sharo-daemon/tests/daemon_ipc.rs`, `crates/sharo-daemon/tests/scenario_a.rs`
Re-run: `cargo test -p sharo-daemon --test daemon_ipc --test scenario_a -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated

### Task 2: Spawn per-connection request tasks and isolate shared state

**Files:**

- Modify: `crates/sharo-daemon/src/main.rs`
- Modify: `crates/sharo-daemon/src/kernel.rs`
- Modify: `crates/sharo-daemon/src/store.rs`

**Preconditions**

- Responsiveness regression tests fail.

**Invariants**

- No store guard is held across `.await`.
- Blocking provider work remains off the Tokio request thread.

**Postconditions**

- Accept loop continues while request handlers run concurrently.

**Tests (must exist before implementation)**

Unit:
- `handle_request_avoids_holding_store_lock_across_provider_work`

Property:
- `serve_many_requests_returns_exactly_one_response_each`

Integration:
- `status_request_remains_responsive_during_slow_submit`
- `approval_list_remains_responsive_during_slow_submit`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-daemon --test daemon_ipc -- --nocapture`
Expected: new concurrency tests fail until the request loop is split.

**Implementation Steps**

1. Wrap store and any request-shared runtime state in explicit shared ownership.
2. Spawn a Tokio task per accepted connection instead of awaiting `handle_stream()` inline.
3. Narrow mutation windows so state access happens before or after async/provider boundaries, not across them.
4. Revisit type shapes if needed to replace stringly lock choreography with narrower helper APIs.

**Green Phase (required)**

Command: `cargo test -p sharo-daemon --test daemon_ipc -- --nocapture`
Expected: daemon IPC tests pass with concurrent request service.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/sharo-daemon/src/main.rs`, `crates/sharo-daemon/src/kernel.rs`, `crates/sharo-daemon/src/store.rs`
Re-run: `cargo test -p sharo-daemon -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated

### Task 3: Verify no regression in smoke and protocol flows

**Files:**

- Modify: `crates/sharo-daemon/tests/daemon_smoke.rs`
- Modify: `crates/sharo-cli/tests/cli_smoke.rs`

**Preconditions**

- Concurrent serving is implemented.

**Invariants**

- CLI and daemon protocol output stays machine-parseable.
- Existing smoke modes keep working.

**Postconditions**

- Baseline smoke coverage remains green after daemon concurrency changes.

**Tests (must exist before implementation)**

Unit:
- `handle_request_avoids_holding_store_lock_across_provider_work`

Property:
- `serve_many_requests_returns_exactly_one_response_each`

Integration:
- `status_request_remains_responsive_during_slow_submit`
- `approval_list_remains_responsive_during_slow_submit`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-daemon --test daemon_smoke && cargo test -p sharo-cli --test cli_smoke`
Expected: stays green before and after the concurrency work; failures indicate a regression in baseline transport behavior.

**Implementation Steps**

1. Run smoke suites after the concurrency refactor.
2. Adjust brittle sequencing assumptions in tests only if the protocol contract still holds.
3. Keep test changes minimal and evidence-oriented.

**Green Phase (required)**

Command: `cargo test -p sharo-daemon --test daemon_smoke && cargo test -p sharo-cli --test cli_smoke`
Expected: smoke tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: smoke tests only
Re-run: `scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated
