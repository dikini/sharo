# Rust Workspace Bootstrap Implementation Plan

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Property, and Integration tests.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: bootstrap a Rust workspace with daemon, CLI, and shared core crates and prove a compiling deterministic vertical slice.
Architecture: use a root Cargo workspace with crates under `crates/`. Keep contracts in `sharo-core` and wire `sharo-cli` plus `sharo-daemon` to those contracts without introducing real IPC yet. Update policy checks to validate workspace member manifests.
Tech Stack: Rust 2024, Tokio, Clap, Serde, shell policy scripts.
Template-Profile: tdd-strict-v1

---

### Task 1: Scaffold Workspace and Crates

**Files:**

- Create: `Cargo.toml`
- Create: `crates/sharo-core/Cargo.toml`
- Create: `crates/sharo-core/src/lib.rs`
- Create: `crates/sharo-cli/Cargo.toml`
- Create: `crates/sharo-cli/src/main.rs`
- Create: `crates/sharo-daemon/Cargo.toml`
- Create: `crates/sharo-daemon/src/main.rs`

**Preconditions**

- [ ] No existing Rust workspace files exist.

**Invariants**

- [ ] Member crates are under `crates/`.
- [ ] Every crate manifest uses `edition = "2024"` and `rust-version >= "1.93"`.

**Postconditions**

- [ ] `cargo check --workspace` includes all bootstrap crates.

**Tests (must exist before implementation)**

Unit:
- [ ] `workspace_manifest_lists_expected_members`

Property:
- [ ] `crate_manifests_include_required_rust_policy_fields`

Integration:
- [ ] `cargo_check_workspace_compiles`

**Red Phase (required before code changes)**

Command: `cargo check --workspace`
Expected: fails because workspace does not yet exist.

**Implementation Steps**

1. Create root workspace `Cargo.toml` with member list and resolver.
2. Create three crate manifests and minimal sources.
3. Run `cargo check --workspace` and capture first green baseline.

**Green Phase (required)**

Command: `cargo check --workspace`
Expected: succeeds for all member crates.

**Refactor Phase (optional but controlled)**

Allowed scope: manifest ordering and crate metadata wording.
Re-run: `cargo check --workspace`

**Completion Evidence**

- [ ] Preconditions satisfied
- [ ] Invariants preserved
- [ ] Postconditions met
- [ ] Unit, property, and integration tests passing
- [ ] CHANGELOG.md updated

### Task 2: Build Core Protocol and Deterministic Stub

**Files:**

- Modify: `crates/sharo-core/src/lib.rs`
- Create: `crates/sharo-core/src/protocol.rs`
- Create: `crates/sharo-core/src/client.rs`
- Create: `crates/sharo-core/tests/protocol_tests.rs`
- Create: `crates/sharo-core/tests/stub_client_tests.rs`

**Preconditions**

- [ ] `sharo-core` compiles as library crate.

**Invariants**

- [ ] Protocol types live in `sharo-core` only.
- [ ] Stub client behavior is deterministic for same input.

**Postconditions**

- [ ] `sharo-core` exposes protocol and stub client APIs consumed by CLI.

**Tests (must exist before implementation)**

Unit:
- [ ] `submit_request_response_roundtrip`
- [ ] `status_request_response_roundtrip`

Property:
- [ ] `stub_submit_is_deterministic_for_goal_and_session`

Integration:
- [ ] `core_public_api_exports_protocol_and_client`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-core`
Expected: fails because tests and APIs are missing.

**Implementation Steps**

1. Add protocol and client modules with request/response/task-state types.
2. Add deterministic `StubClient` implementation.
3. Add tests first, then minimal implementation to pass.

**Green Phase (required)**

Command: `cargo test -p sharo-core`
Expected: all core tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: naming and module split only.
Re-run: `cargo test -p sharo-core`

**Completion Evidence**

- [ ] Preconditions satisfied
- [ ] Invariants preserved
- [ ] Postconditions met
- [ ] Unit, property, and integration tests passing
- [ ] CHANGELOG.md updated

### Task 3: Implement CLI and Daemon Vertical Slice

**Files:**

- Modify: `crates/sharo-cli/src/main.rs`
- Modify: `crates/sharo-daemon/src/main.rs`
- Optionally create: `crates/sharo-cli/src/commands.rs`

**Preconditions**

- [ ] `sharo-core` protocol and stub client compile.

**Invariants**

- [ ] CLI command contracts stay stable (`submit`, `status`).
- [ ] Daemon starts and terminates cleanly.

**Postconditions**

- [ ] CLI commands return deterministic output through `sharo-core` stub.
- [ ] Daemon start path runs under Tokio runtime.

**Tests (must exist before implementation)**

Unit:
- [ ] `cli_submit_parsing_accepts_goal`
- [ ] `cli_status_parsing_requires_task_id`

Property:
- [ ] `status_output_is_deterministic_for_task_id`

Integration:
- [ ] `cli_submit_and_status_smoke`
- [ ] `daemon_start_smoke`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-cli && cargo test -p sharo-daemon`
Expected: fails before command behavior exists.

**Implementation Steps**

1. Add Clap command parsing in CLI and wire to `StubClient`.
2. Add Tokio daemon startup/shutdown baseline.
3. Add tests first, then minimal code until green.

**Green Phase (required)**

Command: `cargo test -p sharo-cli && cargo test -p sharo-daemon && cargo check --workspace`
Expected: all pass.

**Refactor Phase (optional but controlled)**

Allowed scope: output formatting and command helper extraction only.
Re-run: `cargo test -p sharo-cli && cargo test -p sharo-daemon`

**Completion Evidence**

- [ ] Preconditions satisfied
- [ ] Invariants preserved
- [ ] Postconditions met
- [ ] Unit, property, and integration tests passing
- [ ] CHANGELOG.md updated

### Task 4: Update Rust Policy Script for Workspace Members

**Files:**

- Modify: `scripts/check-rust-policy.sh`
- Create/Modify: `scripts/tests/test-rust-policy.sh`

**Preconditions**

- [ ] Workspace and member crate manifests exist.

**Invariants**

- [ ] Script fails closed on manifest violations.
- [ ] Script behavior is deterministic for same inputs.

**Postconditions**

- [ ] Policy script validates all workspace members, not only root package mode.

**Tests (must exist before implementation)**

Unit:
- [ ] `rust_policy_fails_member_bad_edition`
- [ ] `rust_policy_fails_member_low_rust_version`

Property:
- [ ] `rust_policy_workspace_results_are_stable`

Integration:
- [ ] `rust_policy_passes_bootstrap_workspace`

**Red Phase (required before code changes)**

Command: `scripts/check-rust-policy.sh`
Expected: fails or mis-validates in workspace mode before script update.

**Implementation Steps**

1. Extend script to detect workspace member manifests.
2. Validate required fields for each member crate.
3. Add tests/fixtures for member pass/fail cases.

**Green Phase (required)**

Command: `scripts/check-rust-policy.sh`
Expected: passes for valid workspace and fails with actionable message for invalid fixtures.

**Refactor Phase (optional but controlled)**

Allowed scope: diagnostics and helper function structure.
Re-run: `scripts/check-rust-policy.sh`

**Completion Evidence**

- [ ] Preconditions satisfied
- [ ] Invariants preserved
- [ ] Postconditions met
- [ ] Unit, property, and integration tests passing
- [ ] CHANGELOG.md updated

### Task 5: Policy/Task/Docs Finalization

**Files:**

- Modify: `docs/tasks/tasks.csv`
- Modify: `docs/tasks/README.md`
- Modify: `CHANGELOG.md`

**Preconditions**

- [ ] Tasks 1-4 are green.

**Invariants**

- [ ] Task registry remains valid and source-referenced.

**Postconditions**

- [ ] Workspace bootstrap tracked as done.
- [ ] Deferred transport/runtime expansion tracked explicitly if needed.

**Tests (must exist before implementation)**

Unit:
- [ ] `tasks_registry_rows_use_valid_status`

Property:
- [ ] `task_ids_are_referenced_in_source_docs`

Integration:
- [ ] `precommit_policy_checks_pass_with_workspace_changes`

**Red Phase (required before code changes)**

Command: `scripts/check-tasks-registry.sh && scripts/check-tasks-sync.sh --changed`
Expected: may fail until task registry and docs are updated.

**Implementation Steps**

1. Update task rows for workspace task status.
2. Update changelog with workspace and policy-script changes.
3. Run full project checks.

**Green Phase (required)**

Command: `scripts/doc-lint.sh --changed --strict-new && scripts/check-doc-terms.sh --changed && scripts/check-tasks-registry.sh && scripts/check-tasks-sync.sh --changed && scripts/check-rust-policy.sh && cargo test --workspace`
Expected: all checks pass.

**Refactor Phase (optional but controlled)**

Allowed scope: wording in docs and changelog entries.
Re-run: `scripts/doc-lint.sh --changed --strict-new`

**Completion Evidence**

- [ ] Preconditions satisfied
- [ ] Invariants preserved
- [ ] Postconditions met
- [ ] Unit, property, and integration tests passing
- [ ] CHANGELOG.md updated
