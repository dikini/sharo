# MVP Slice 004 Protocol And CLI Completion Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: complete required MVP protocol operations and CLI command surface.
Architecture: extend protocol and CLI in incremental command groups while preserving compatibility and deterministic output. Mutation operations must return explicit acceptance state, stable ids, and rejection reason.
Tech Stack: Rust 2024, clap, serde, daemon protocol.
Template-Profile: tdd-strict-v1

---

### Task 1: Complete Required Protocol Operations

**Files:**
- Modify: `crates/sharo-core/src/protocol.rs`
- Test: `crates/sharo-core/tests/protocol_surface_tests.rs`

**Preconditions**
- Scenario slices 001-003 protocol additions are present.

**Invariants**
- Protocol shape remains explicit and serializable.

**Postconditions**
- All required operations are represented in request and response envelopes.

**Tests (must exist before implementation)**

Unit:
- `protocol_contains_required_mvp_operations`

Property:
- `mutation_response_contains_acceptance_and_reason_fields`

Integration:
- `protocol_envelope_roundtrip_for_all_operations`

**Red Phase (required before code changes)**

Command: `cargo test --package sharo-core protocol_surface_tests -- --nocapture`
Expected: fails before operation set is complete.

**Implementation Steps**

1. Add missing operation payload structs and envelope variants.
2. Ensure stable identifiers and reason fields are part of mutation outputs.

**Green Phase (required)**

Command: `cargo test --workspace`
Expected: tests pass.

### Task 2: Add CLI Commands For Required MVP Surface

**Files:**
- Modify: `crates/sharo-cli/src/main.rs`
- Test: `crates/sharo-cli/tests/cli_surface_tests.rs`

**Preconditions**
- Protocol operations are available in core crate.

**Invariants**
- CLI command hierarchy remains machine-parseable and stable.

**Postconditions**
- CLI supports required groups: `session`, `task`, `trace`, `artifacts`, `approval`, `daemon`.

**Tests (must exist before implementation)**

Unit:
- `cli_surface_has_required_command_groups`

Property:
- `cli_read_commands_are_side_effect_free`

Integration:
- `cli_command_set_maps_to_protocol_operations`

**Red Phase (required before code changes)**

Command: `cargo test --package sharo-cli cli_surface_tests -- --nocapture`
Expected: fails before command set is complete.

**Implementation Steps**

1. Add missing commands and flags.
2. Map all commands to daemon protocol operations.

**Green Phase (required)**

Command: `cargo test --workspace`
Expected: workspace tests pass.

### Task 3: Add Idempotency And Control Semantics

**Files:**
- Modify: `crates/sharo-daemon/src/main.rs`
- Modify: `crates/sharo-daemon/src/store.rs`
- Test: `crates/sharo-daemon/tests/idempotency_and_control.rs`

**Preconditions**
- Protocol and CLI surfaces are complete.

**Invariants**
- `submit-task` idempotency keys are replay-safe.

**Postconditions**
- `submit-task` supports idempotency key and `control-task` supports cancellation with explicit state updates.

**Tests (must exist before implementation)**

Unit:
- `submit_idempotency_key_reuses_task_id`

Property:
- `replayed_reads_have_no_side_effect`

Integration:
- `task_cancel_flow_is_visible_and_durable`

**Red Phase (required before code changes)**

Command: `cargo test --package sharo-daemon idempotency_and_control -- --nocapture`
Expected: fails before idempotency and control are implemented.

**Implementation Steps**

1. Store and enforce idempotency mapping.
2. Add task cancel control handling and response summaries.

**Green Phase (required)**

Command: `cargo test --workspace`
Expected: workspace tests pass.

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated
