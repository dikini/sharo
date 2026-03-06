# Daemon Submit Parallelism Implementation Plan

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Property, and Integration tests.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: allow independent provider-backed submit requests to run concurrently without breaking idempotent commit behavior.
Architecture: remove the process-wide submit mutex and rely on short store critical sections plus existing transactional commit/replay behavior. Prove the change with a daemon scenario that measures upstream overlap rather than only local timing.
Tech Stack: Rust 2024, Tokio, daemon scenario tests, transactional store.
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-DAEMON-SUBMIT-PARALLELISM-SPEC-001, TASK-DAEMON-SUBMIT-PARALLELISM-PLAN-001

---

### Task 1: Add failing overlap coverage for submit requests

**Files:**

- Modify: `crates/sharo-daemon/tests/scenario_a.rs`
- Modify: `crates/sharo-daemon/src/main.rs`

**Preconditions**

- Current submit and responsiveness scenarios pass.

**Invariants**

- The new test measures concurrent upstream progress, not only wall-clock completion.
- Existing idempotency scenarios remain unchanged.

**Postconditions**

- There is a regression test that fails while submit requests are globally serialized.

**Tests (must exist before implementation)**

Unit:
- `submit_requests_do_not_share_process_wide_guard`

Property:
- `independent_submit_requests_can_progress_in_parallel`

Integration:
- `concurrent_slow_submits_make_parallel_upstream_progress`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-daemon --test scenario_a concurrent_slow_submits_make_parallel_upstream_progress -- --nocapture`
Expected: fails while submits are serialized behind the process-wide guard.

**Implementation Steps**

1. Add a local HTTP stub that tracks max concurrent upstream requests.
2. Drive two slow `SubmitTask` requests concurrently through the daemon.
3. Assert observed upstream concurrency is greater than one for independent submits.

**Green Phase (required)**

Command: `cargo test -p sharo-daemon --test scenario_a concurrent_slow_submits_make_parallel_upstream_progress -- --nocapture`
Expected: overlap scenario passes.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/sharo-daemon/src/main.rs`, `crates/sharo-daemon/tests/scenario_a.rs`
Re-run: `cargo test -p sharo-daemon --test scenario_a -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated
