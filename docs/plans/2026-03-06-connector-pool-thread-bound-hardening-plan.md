# Connector Pool Thread Bound Hardening Implementation Plan

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Property, and Integration tests.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: make connector-pool scale-up honor `max_threads` under real concurrent submission races.
Architecture: keep the existing pool, but replace the advisory load-then-spawn path with an atomic reservation path so worker creation is bounded before thread spawn occurs. Follow Rust-skill guidance around bounded concurrency and race-focused testing instead of relying on sequential unit coverage.
Tech Stack: Rust 2024, `std::sync::atomic`, `crossbeam-channel`, daemon unit/integration tests.
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-CONNECTOR-POOL-HARDENING-PLAN-001, TASK-CONNECTOR-POOL-HARDENING-SPEC-001

---

### Task 1: Add failing race coverage for scale-up

**Files:**

- Modify: `crates/sharo-daemon/src/connector_pool.rs`
- Test: `crates/sharo-daemon/src/connector_pool.rs`

**Preconditions**

- Existing pool scaling tests pass in the current tree.

**Invariants**

- New tests target only worker-bound behavior.
- Existing overload and cooldown semantics remain asserted.

**Postconditions**

- There is a deterministic test that fails while the current race exists.

**Tests (must exist before implementation)**

Unit:
- `scale_up_reservation_is_atomic_under_race`

Property:
- `worker_count_never_exceeds_max_threads_under_parallel_submit`

Integration:
- `daemon_burst_submit_never_exceeds_configured_connector_workers`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-daemon scale_up_reservation_is_atomic_under_race worker_count_never_exceeds_max_threads_under_parallel_submit -- --nocapture`
Expected: the new tests fail against the current non-atomic scale-up path.

**Implementation Steps**

1. Add a focused race harness that coordinates parallel `submit()` calls.
2. Add a worker-bound assertion that inspects `current_worker_count()` after pressure stabilizes.
3. Keep the test narrow enough to isolate the bound violation.

**Green Phase (required)**

Command: `cargo test -p sharo-daemon scale_up_reservation_is_atomic_under_race worker_count_never_exceeds_max_threads_under_parallel_submit -- --nocapture`
Expected: the new race tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/sharo-daemon/src/connector_pool.rs`
Re-run: `cargo test -p sharo-daemon connector_pool::tests -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated

### Task 2: Replace advisory scale-up with atomic reservation

**Files:**

- Modify: `crates/sharo-daemon/src/connector_pool.rs`

**Preconditions**

- Task 1 coverage is failing and reproduces the issue.

**Invariants**

- `active_workers` accounting is updated through a single reservation path.
- Worker spawn failure or early exit does not leak count.

**Postconditions**

- Worker creation cannot exceed `max_threads`.

**Tests (must exist before implementation)**

Unit:
- `scale_up_reservation_is_atomic_under_race`

Property:
- `worker_count_never_exceeds_max_threads_under_parallel_submit`

Integration:
- `daemon_burst_submit_never_exceeds_configured_connector_workers`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-daemon connector_pool::tests -- --nocapture`
Expected: Task 1 race tests fail before the reservation change.

**Implementation Steps**

1. Introduce an atomic compare-and-swap or equivalent reservation helper for worker creation.
2. Move the worker-count increment into the reservation step rather than `spawn_worker()`.
3. Ensure timeout/disconnect scale-down paths release the reserved active count exactly once.
4. Re-check `maybe_scale_up()` against `rust-skills` guidance for bounded channels and shared ownership.

**Green Phase (required)**

Command: `cargo test -p sharo-daemon connector_pool::tests -- --nocapture`
Expected: all pool tests pass, including the new race coverage.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/sharo-daemon/src/connector_pool.rs`
Re-run: `cargo test -p sharo-daemon -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated

### Task 3: Validate daemon-level behavior under burst load

**Files:**

- Modify: `crates/sharo-daemon/tests/scenario_a.rs`

**Preconditions**

- Pool race fix is green in unit coverage.

**Invariants**

- Daemon IPC and connector routing semantics stay unchanged.
- Test setup remains deterministic.

**Postconditions**

- End-to-end daemon coverage proves worker bounds remain enforced under load.

**Tests (must exist before implementation)**

Unit:
- `scale_up_reservation_is_atomic_under_race`

Property:
- `worker_count_never_exceeds_max_threads_under_parallel_submit`

Integration:
- `daemon_burst_submit_never_exceeds_configured_connector_workers`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-daemon --test scenario_a daemon_burst_submit_never_exceeds_configured_connector_workers -- --nocapture`
Expected: fails until the scenario is added.

**Implementation Steps**

1. Add a daemon scenario that drives concurrent load with a small max-worker config.
2. Expose or assert the bounded-worker outcome through daemon-visible evidence.
3. Keep the scenario focused on the fixed regression rather than generic throughput.

**Green Phase (required)**

Command: `cargo test -p sharo-daemon --test scenario_a daemon_burst_submit_never_exceeds_configured_connector_workers -- --nocapture`
Expected: new integration coverage passes.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/sharo-daemon/tests/scenario_a.rs`
Re-run: `scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated
