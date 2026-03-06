# Daemon Blocking Submit Offload Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** move provider-backed submit execution off Tokio runtime worker threads while preserving daemon submit semantics.

**Architecture:** keep the async Unix socket front-end in Tokio, but hand synchronous submit execution to an explicit blocking boundary. Preserve the existing store preparation and commit flow, and prove the change through daemon IPC responsiveness tests under concurrent slow-submit pressure.

**Tech Stack:** Rust 2024, Tokio `spawn_blocking`, daemon IPC integration tests, transactional store.

---

Template-Profile: tdd-strict-v1
Task-Registry-Refs: TASK-DAEMON-BLOCKING-OFFLOAD-SPEC-001, TASK-DAEMON-BLOCKING-OFFLOAD-PLAN-001

### Task 1: Offload blocking submit execution

**Files:**
- Modify: `crates/sharo-daemon/tests/daemon_ipc.rs`
- Modify: `crates/sharo-daemon/src/main.rs`

**Preconditions**

- Existing daemon IPC and submit scenarios pass.

**Invariants**

- Submit replay and persistence semantics remain unchanged.
- The request task retains response ownership.

**Postconditions**

- Blocking submit work runs behind an explicit blocking boundary.
- A status request stays responsive while multiple slow submits are in flight.

**Tests (must exist before implementation)**

Unit:
- `submit_execution_runs_outside_runtime_worker`

Property:
- `runtime_workers_remain_available_under_slow_submit_pressure`

Integration:
- `status_requests_remain_responsive_under_parallel_slow_submits`

**Red Phase (required before code changes)**

Run: `cargo test -p sharo-daemon --test daemon_ipc status_requests_remain_responsive_under_parallel_slow_submits -- --exact --nocapture`
Expected: FAIL because synchronous submit execution still occupies runtime worker threads.

**Implementation Steps**

1. Add the failing daemon IPC test that starts enough slow submits to expose runtime thread starvation.
2. Route blocking `SubmitTask` execution through `tokio::task::spawn_blocking`.
3. Keep the request task responsible for awaiting the blocking result and writing the response.
4. Re-run the focused daemon IPC test and then the daemon crate test suite.

**Green Phase (required)**

Run: `cargo test -p sharo-daemon --test daemon_ipc status_requests_remain_responsive_under_parallel_slow_submits -- --exact --nocapture`
Expected: PASS.

**Completion Evidence**

- Focused red/green test recorded
- `cargo test -p sharo-daemon` passes
- `scripts/check-fast-feedback.sh` passes
- `CHANGELOG.md` updated
