# Rust Workspace Bootstrap

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Define tests before implementation work.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Property, and Integration tests.
> - Run `scripts/doc-lint.sh` before commit.

Updated: 2026-03-04
Status: active
Owner: sharo-core
Template-Profile: tdd-strict-v1

## Purpose

Establish a Rust workspace baseline for Sharo with separate daemon and CLI binaries plus a shared core crate, enabling a deterministic compiling vertical slice that can evolve into the MVP runtime.

## Scope

### In Scope

- Root Cargo workspace with member crates under `crates/`.
- Crates: `sharo-daemon`, `sharo-cli`, `sharo-core`.
- Rust policy compliance in all crate manifests: `edition = "2024"`, `rust-version >= "1.93"`.
- Typed protocol contracts in `sharo-core`.
- Deterministic stub client in `sharo-core` for initial CLI-to-runtime path.
- Tokio runtime baseline for daemon startup.
- Clap command baseline for CLI task submit/status operations.
- Workspace-aware Rust policy check script behavior.

### Out of Scope

- Production IPC transport between CLI and daemon.
- Persistence, approvals, policy engine, and capability execution.
- Knot bridge or external sync integration in runtime.
- TUI and non-CLI surfaces.

## Core Terms

- `Workspace`: root Cargo workspace coordinating all member crates.
- `Core Crate`: shared library (`sharo-core`) containing protocol types and client abstractions.
- `Stub Client`: deterministic local implementation used before real transport.
- `Vertical Slice`: minimal end-to-end operation proving architecture wiring without full subsystem depth.

## Interfaces / Contracts

- Crate names and roles:
  - `sharo-core`: protocol module and runtime-client interface.
  - `sharo-cli`: operator command surface.
  - `sharo-daemon`: daemon process entry point and runtime loop baseline.
- CLI contract:
  - `submit --goal <text> [--session-id <id>]`
  - `status --task-id <id>`
- Core type contract (minimum):
  - `TaskState` enum including `Submitted`, `Running`, `Succeeded`, `Failed`, `Blocked`.
  - `SubmitTaskRequest`, `SubmitTaskResponse`.
  - `TaskStatusRequest`, `TaskStatusResponse`.

## Invariants

- Workspace contains exactly the three bootstrap crates listed above.
- All crate manifests satisfy project Rust policy.
- Shared request/response types live in `sharo-core`, not duplicated in binaries.
- CLI output for stub-backed commands is deterministic for the same input.
- Rust policy checks fail closed for non-compliant member crates.

## Task Contracts

### Task 1: Workspace and Crate Scaffolding

**Preconditions**

- Repository has no existing `Cargo.toml` workspace.
- Existing docs/policy hooks remain intact.

**Invariants**

- Crate paths are under `crates/`.
- Workspace member names are stable and prefixed with `sharo-`.

**Postconditions**

- `cargo check --workspace` executes and includes all three crates.

**Tests (must exist before implementation)**

Unit:
- `workspace_manifest_lists_expected_members`

Property:
- `all_member_manifests_enforce_rust_policy_fields`

Integration:
- `cargo_check_workspace_compiles_bootstrap_crates`

### Task 2: Shared Core Protocol and Stub Client

**Preconditions**

- `sharo-core` crate exists and compiles.

**Invariants**

- Request/response contracts are crate-public and serializable.
- Stub responses are deterministic and side-effect-free.

**Postconditions**

- CLI can use `sharo-core` client API without local duplicated protocol types.

**Tests (must exist before implementation)**

Unit:
- `submit_request_response_roundtrip`
- `status_request_response_roundtrip`

Property:
- `stub_status_is_deterministic_for_task_id`

Integration:
- `cli_commands_use_core_protocol_types`

### Task 3: Daemon and CLI Vertical Slice

**Preconditions**

- `sharo-core` protocol and stub client are available.

**Invariants**

- Daemon can start and shut down cleanly.
- CLI command parsing is explicit and stable.

**Postconditions**

- CLI `submit` and `status` commands execute end-to-end against stub client and print deterministic output.

**Tests (must exist before implementation)**

Unit:
- `cli_submit_parsing_accepts_goal`
- `cli_status_parsing_requires_task_id`

Property:
- `submit_output_contains_task_id_and_state`

Integration:
- `daemon_start_command_runs_until_interrupt`
- `cli_submit_and_status_smoke`

### Task 4: Policy Script Alignment for Workspace

**Preconditions**

- Root workspace manifest and member manifests exist.

**Invariants**

- Rust policy checker behavior is deterministic and fail-closed.

**Postconditions**

- `scripts/check-rust-policy.sh` validates workspace member crates.

**Tests (must exist before implementation)**

Unit:
- `rust_policy_rejects_member_with_bad_edition`
- `rust_policy_rejects_member_with_low_rust_version`

Property:
- `rust_policy_results_are_stable_for_same_workspace`

Integration:
- `rust_policy_passes_for_bootstrap_workspace`

## Scenarios

1. Build baseline:
- Run `cargo check --workspace` in a clean checkout.
- All member crates compile.

2. Operator submits task from CLI:
- `sharo submit --goal "read docs"` returns deterministic `task_id` and `state`.

3. Operator checks task status:
- `sharo status --task-id task-0001` returns deterministic state and summary.

4. Policy enforcement:
- Rust policy script passes for valid workspace, fails when a member manifest breaks edition/version policy.

## Verification

- `cargo check --workspace`
- `cargo test --workspace`
- `scripts/check-rust-policy.sh`
- `scripts/doc-lint.sh --changed --strict-new`
- `scripts/check-doc-terms.sh --changed`
- `scripts/check-tasks-registry.sh`
- `scripts/check-tasks-sync.sh --changed`

## Risks and Failure Modes

- Script assumptions built for single-package Cargo layouts may miss member validation.
- Early over-design of transport can slow bootstrap progress.
- Inconsistent protocol type placement can cause duplicate contracts across crates.

## Open Questions

- Should daemon command naming stay `start` only in v1, or include explicit `run` alias?
- Should stub client later live behind Cargo feature flags when real transport is added?

## References

- [AGENTS.md](/home/dikini/Projects/sharo/AGENTS.md)
- [mvp.md](/home/dikini/Projects/sharo/docs/specs/mvp.md)
- [tasks.csv](/home/dikini/Projects/sharo/docs/tasks/tasks.csv)
