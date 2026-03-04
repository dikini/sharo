# Ipc Transport

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Define tests before implementation work.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Property, and Integration tests.
> - Run `scripts/doc-lint.sh` before commit.

Updated: 2026-03-05
Status: active
Owner: sharo-runtime
Template-Profile: tdd-strict-v1

## Purpose

Replace the bootstrap stub-only CLI path with a real local IPC transport between `sharo-cli` and `sharo-daemon` while keeping deterministic behavior and narrow scope.

## Scope

### In Scope

- Local Unix domain socket transport.
- JSON request/response envelope types in `sharo-core`.
- CLI transport selection with IPC default and explicit stub fallback.
- Daemon request loop serving `submit` and `status`.
- Integration tests proving CLI↔daemon round-trip behavior.

### Out of Scope

- Remote transport or authentication.
- Persistent task store.
- Multi-request session state beyond deterministic stub-backed handler logic.

## Core Terms

- `IPC Request`: serialized daemon request envelope over Unix socket.
- `IPC Response`: serialized daemon response envelope over Unix socket.
- `Serve Once`: daemon mode that handles one request then exits.

## Interfaces / Contracts

- Socket path default: `/tmp/sharo-daemon.sock`.
- CLI global flags:
  - `--transport <ipc|stub>` (default `ipc`)
  - `--socket-path <path>` (used for `ipc` transport)
- Daemon flags on `start`:
  - `--socket-path <path>`
  - `--serve-once`
  - `--once` (immediate boot/shutdown smoke mode)
- Core IPC envelope types:
  - `DaemonRequest::{Submit, Status}`
  - `DaemonResponse::{Submit, Status, Error}`

## Invariants

- IPC contracts are defined in `sharo-core` and shared by CLI and daemon.
- CLI transport mode is explicit and deterministic.
- Daemon responses are exactly one response per request.
- IPC failures return non-zero CLI exit with actionable error text.

## Task Contracts

### Task 1: Shared IPC Envelope Types

**Preconditions**

- `sharo-core` protocol types exist.

**Invariants**

- Envelope variants map 1:1 with CLI commands.

**Postconditions**

- `sharo-core` exports request/response envelope enums for IPC.

**Tests (must exist before implementation)**

Unit:
- `ipc_submit_envelope_roundtrip`
- `ipc_status_envelope_roundtrip`

Property:
- `response_variant_matches_request_kind`

Integration:
- `core_exports_ipc_envelopes`

### Task 2: Daemon Socket Server

**Preconditions**

- Shared IPC envelopes are available.

**Invariants**

- One request yields one response.
- Invalid payload yields `Error` response.

**Postconditions**

- Daemon serves submit/status requests over Unix socket.

**Tests (must exist before implementation)**

Unit:
- `daemon_handle_submit_request`
- `daemon_handle_status_request`

Property:
- `serve_once_handles_exactly_one_request`

Integration:
- `daemon_ipc_submit_roundtrip`

### Task 3: CLI IPC Client

**Preconditions**

- Daemon server contract is stable.

**Invariants**

- IPC transport is default.
- Stub transport remains available for deterministic fallback.

**Postconditions**

- CLI submit/status work via IPC when daemon is available.

**Tests (must exist before implementation)**

Unit:
- `cli_transport_flag_parses_values`

Property:
- `ipc_submit_output_contains_task_and_state`

Integration:
- `cli_submit_status_against_daemon_socket`

## Scenarios

1. IPC submit:
- Start daemon on temp socket with `--serve-once`.
- Run `sharo --transport ipc --socket-path <socket> submit --goal ...`.
- Request succeeds with deterministic output.

2. IPC status:
- Start daemon on temp socket with `--serve-once`.
- Run `sharo --transport ipc --socket-path <socket> status --task-id ...`.
- Request succeeds with deterministic state and summary.

3. Missing daemon socket:
- Run CLI in IPC mode against nonexistent socket.
- CLI exits non-zero with connection error.

## Verification

- `cargo test --workspace`
- `scripts/check-rust-policy.sh`
- `scripts/check-rust-tests.sh --all`
- `scripts/check-fast-feedback.sh --all`

## Risks and Failure Modes

- Socket path collisions from stale files.
- Partial read/write framing errors if newline framing is not enforced.
- Cross-platform behavior differences for Unix sockets.

## Open Questions

- Should socket default move to XDG runtime dir after MVP bootstrap?

## References

- [rust-workspace-bootstrap.md](/home/dikini/Projects/sharo/docs/specs/rust-workspace-bootstrap.md)
- [tasks.csv](/home/dikini/Projects/sharo/docs/tasks/tasks.csv)
