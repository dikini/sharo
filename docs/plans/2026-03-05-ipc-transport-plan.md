# Ipc Transport Implementation Plan

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Property, and Integration tests.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: replace stub-only CLI execution path with local daemon IPC transport while preserving deterministic behavior and strict policy checks.
Architecture: define IPC envelopes in `sharo-core`, implement Unix socket request loop in daemon, and implement IPC client in CLI with `ipc` default and explicit `stub` fallback.
Tech Stack: Rust 2024, Tokio Unix sockets, Serde JSON, Clap.
Template-Profile: tdd-strict-v1

---

### Task 1: Add shared IPC envelopes in core

**Files:**

- Modify: `crates/sharo-core/src/protocol.rs`
- Create: `crates/sharo-core/tests/ipc_protocol_tests.rs`

**Preconditions**

- `sharo-core` protocol module compiles.

**Invariants**

- Request/response envelopes align with submit/status command semantics.

**Postconditions**

- Core exports `DaemonRequest` and `DaemonResponse` enums.

**Tests (must exist before implementation)**

Unit:
- `ipc_submit_envelope_roundtrip`
- `ipc_status_envelope_roundtrip`

Property:
- `response_variant_matches_request_kind`

Integration:
- `core_exports_ipc_envelopes`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-core ipc_`
Expected: fails because IPC envelope tests and types do not exist.

**Implementation Steps**

1. Add test file for IPC envelope roundtrip behavior.
2. Add request/response envelope enums to protocol module.
3. Run focused core tests until green.

**Green Phase (required)**

Command: `cargo test -p sharo-core ipc_`
Expected: IPC envelope tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: enum naming and module organization.
Re-run: `cargo test -p sharo-core`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated

### Task 2: Implement daemon Unix socket server

**Files:**

- Modify: `crates/sharo-daemon/Cargo.toml`
- Modify: `crates/sharo-daemon/src/main.rs`
- Modify: `crates/sharo-daemon/tests/daemon_smoke.rs`

**Preconditions**

- IPC envelope types are available in core.

**Invariants**

- One request returns one response.
- Daemon handles malformed JSON with error response.

**Postconditions**

- Daemon serves requests on socket path and supports `--serve-once`.

**Tests (must exist before implementation)**

Unit:
- `daemon_handle_submit_request`

Property:
- `serve_once_exits_after_first_request`

Integration:
- `daemon_start_smoke` remains green

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-daemon`
Expected: fails for new server behavior tests.

**Implementation Steps**

1. Add daemon start flags for socket path and serve-once.
2. Implement Tokio UnixListener request loop.
3. Parse request JSON, dispatch through deterministic handler, and write JSON response.

**Green Phase (required)**

Command: `cargo test -p sharo-daemon`
Expected: daemon tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: error message text and helper extraction.
Re-run: `cargo test -p sharo-daemon`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated

### Task 3: Implement CLI IPC client and integration tests

**Files:**

- Modify: `crates/sharo-cli/Cargo.toml`
- Modify: `crates/sharo-cli/src/main.rs`
- Modify: `crates/sharo-cli/tests/cli_smoke.rs`

**Preconditions**

- Daemon socket server path is available.

**Invariants**

- Transport mode is explicit and deterministic.
- IPC mode fails non-zero when connection is unavailable.

**Postconditions**

- CLI submit/status use IPC by default and still support explicit stub fallback.

**Tests (must exist before implementation)**

Unit:
- `cli_transport_flag_parses_values`

Property:
- `ipc_submit_output_contains_task_and_state`

Integration:
- `cli_submit_status_against_daemon_socket`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-cli`
Expected: fails for new IPC integration tests.

**Implementation Steps**

1. Add transport/socket flags and tokio main.
2. Add UnixStream JSON request/response handling in CLI.
3. Update smoke tests to cover stub fallback and IPC roundtrip.

**Green Phase (required)**

Command: `cargo test -p sharo-cli`
Expected: CLI tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: CLI output formatting and helper function extraction.
Re-run: `cargo test -p sharo-cli`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated

### Task 4: Task/changelog completion updates

**Files:**

- Modify: `docs/tasks/tasks.csv`
- Modify: `docs/tasks/README.md`
- Modify: `docs/aliases.toml`
- Modify: `CHANGELOG.md`

**Preconditions**

- IPC implementation/tests are green.

**Invariants**

- Task registry remains source-referenced and status-valid.

**Postconditions**

- `TASK-IPC-TRANSPORT-001` is moved from deferred to done with evidence.

**Tests (must exist before implementation)**

Unit:
- `tasks_registry_rows_use_valid_status`

Property:
- `task_ids_are_referenced_in_source_docs`

Integration:
- `fast_feedback_and_policy_hooks_pass`

**Red Phase (required before code changes)**

Command: `scripts/check-tasks-registry.sh`
Expected: passes before update; task still deferred.

**Implementation Steps**

1. Update task registry and README completion list.
2. Add alias entries for new spec/plan if needed.
3. Update changelog and run full checks.

**Green Phase (required)**

Command: `scripts/check-fast-feedback.sh --all`
Expected: all local policy and test checks pass.

**Refactor Phase (optional but controlled)**

Allowed scope: documentation wording only.
Re-run: `scripts/doc-lint.sh --changed --strict-new`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated
