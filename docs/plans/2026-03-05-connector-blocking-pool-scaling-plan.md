# Connector Blocking Pool Scaling Implementation Plan

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Property, and Integration tests.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: replace per-request connector thread spawning with a bounded worker pool, then add policy-driven adaptive scaling without changing user-facing IPC contracts.
Architecture: introduce a daemon-local connector execution service that owns worker threads and queueing; kernel submits blocking connector jobs to this service and waits for results synchronously. Keep deterministic connector behavior unchanged while OpenAI/Ollama move to pooled execution. Extend the service with an optional scale controller in phase 2.
Tech Stack: Rust 2024, `std::sync::mpsc` or `crossbeam-channel`, daemon config TOML parser, existing sharo core/daemon tests.
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-CONNECTOR-POOL-PLAN-001, TASK-CONNECTOR-POOL-SPEC-001

---

### Task 1: Implement Bounded Connector Worker Pool (Fixed Size)

Task-ID: TASK-CONNECTOR-POOL-IMPL-001

**Files:**

- Create: `crates/sharo-daemon/src/connector_pool.rs`
- Modify: `crates/sharo-daemon/src/kernel.rs`
- Modify: `crates/sharo-daemon/src/main.rs`
- Test: `crates/sharo-daemon/tests/daemon_ipc.rs`

**Preconditions**

- Current daemon can execute OpenAI-compatible turns end to end.
- Connector panics under async runtime are reproducible with blocking reqwest path.

**Invariants**

- No per-request `std::thread::spawn` in connector execution path.
- Per-task/session ordering semantics are preserved.

**Postconditions**

- OpenAI/Ollama turns execute on bounded worker pool.
- Queue full is surfaced as explicit connector overload error.

**Tests (must exist before implementation)**

Unit:
- `pool_reuses_fixed_workers`

Property:
- `worker_count_never_exceeds_configured_max`

Integration:
- `daemon_ipc_submit_roundtrip`
- `daemon_ipc_openai_auth_missing_is_error_envelope`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-daemon --test daemon_ipc -- --nocapture`
Expected: pool-specific tests fail until pool module/wiring exists.

**Implementation Steps**

1. Add `connector_pool` module with fixed-size workers and bounded queue.
2. Inject pool into daemon kernel runtime and route OpenAI/Ollama calls through pool.
3. Add explicit overload mapping for queue saturation.
4. Keep deterministic connector on inline execution path.

**Green Phase (required)**

Command: `cargo test -p sharo-daemon --test daemon_ipc -- --nocapture`
Expected: daemon IPC tests and new pool tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/sharo-daemon/src/kernel.rs`, `crates/sharo-daemon/src/connector_pool.rs`
Re-run: `cargo test -p sharo-daemon --test daemon_ipc`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated

### Task 2: Add Configurable Execution Policy Surface

Task-ID: TASK-CONNECTOR-POOL-IMPL-002

**Files:**

- Modify: `crates/sharo-daemon/src/config.rs`
- Modify: `crates/sharo-daemon/src/kernel.rs`
- Modify: `crates/sharo-daemon/tests/scenario_a.rs`
- Test: `crates/sharo-daemon/src/config.rs`

**Preconditions**

- Task 1 pool exists and is wired to blocking connectors.

**Invariants**

- Default config remains safe and deterministic.
- Invalid policy config fails fast with explicit error.

**Postconditions**

- Daemon TOML supports pool policy fields (`min_threads`, `max_threads`, `queue_capacity`).
- Kernel validates policy bounds (`min <= max`, non-zero queue).

**Tests (must exist before implementation)**

Unit:
- `parse_connector_pool_policy_from_toml`
- `reject_invalid_connector_pool_policy_bounds`

Property:
- `default_policy_values_are_nonzero_and_bounded`

Integration:
- `scenario_a_read_task_succeeds_with_verification_artifact`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-daemon config::tests -- --nocapture`
Expected: new policy tests fail until config surface is implemented.

**Implementation Steps**

1. Extend config model with connector pool policy section.
2. Add runtime validation and defaulting in kernel runtime config build path.
3. Wire validated policy into pool construction.

**Green Phase (required)**

Command: `cargo test -p sharo-daemon -- --nocapture`
Expected: daemon unit and integration tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: config and kernel validation only
Re-run: `cargo test -p sharo-daemon`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated

### Task 3: Implement Adaptive Scaling Controller (Future Phase)

Task-ID: TASK-CONNECTOR-POOL-IMPL-003

**Files:**

- Modify: `crates/sharo-daemon/src/connector_pool.rs`
- Modify: `crates/sharo-daemon/src/config.rs`
- Test: `crates/sharo-daemon/tests/scenario_a.rs`

**Preconditions**

- Tasks 1 and 2 complete with fixed-size pool and validated policy.

**Invariants**

- Scaling stays within configured bounds.
- Scale-down does not interrupt in-flight work.

**Postconditions**

- Pool scales up on sustained queue pressure and scales down on idle windows.
- Scaling decisions are observable through runtime diagnostics.

**Tests (must exist before implementation)**

Unit:
- `scale_up_respects_threshold_and_cooldown`
- `scale_down_respects_idle_window`

Property:
- `scale_state_always_within_min_max_bounds`

Integration:
- `burst_load_scales_then_recovers`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-daemon burst_load_scales_then_recovers -- --nocapture`
Expected: fails until scaling controller is implemented.

**Implementation Steps**

1. Add scale controller loop and telemetry counters.
2. Implement hysteresis using cooldown and threshold settings.
3. Add diagnostics for current worker count and queue depth.

**Green Phase (required)**

Command: `cargo test -p sharo-daemon -- --nocapture`
Expected: all daemon tests pass including scaling tests.

**Refactor Phase (optional but controlled)**

Allowed scope: connector pool internals only
Re-run: `scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated
