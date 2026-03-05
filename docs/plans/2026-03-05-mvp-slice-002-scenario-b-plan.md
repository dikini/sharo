# MVP Slice 002 Scenario B Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: implement restricted execution behavior with policy deny and approval-gated flows.
Architecture: add explicit policy decisions and durable approvals before expanding restricted capability execution. Restricted steps remain fail-closed until policy and approval checks pass.
Tech Stack: Rust 2024, serde, local store, daemon protocol extensions.
Template-Profile: tdd-strict-v1

---

### Task 1: Add Policy Decision Model And Evaluation

**Files:**
- Create: `crates/sharo-core/src/policy.rs`
- Modify: `crates/sharo-daemon/src/main.rs`
- Test: `crates/sharo-core/tests/policy_tests.rs`

**Preconditions**
- Scenario A path is stable.

**Invariants**
- Restricted actions are never executed before policy decision.

**Postconditions**
- Policy returns `allow`, `deny`, or `require_approval` with durable reason fields.

**Tests (must exist before implementation)**

Unit:
- `restricted_action_requires_policy_decision`

Property:
- `policy_decision_is_deterministic_for_same_input`

Integration:
- `denied_restricted_step_does_not_execute`

**Red Phase (required before code changes)**

Command: `cargo test --package sharo-core policy_tests -- --nocapture`
Expected: fails before policy model exists.

**Implementation Steps**

1. Add policy decision structures and evaluation interface.
2. Invoke policy before restricted step execution.

**Green Phase (required)**

Command: `cargo test --workspace`
Expected: policy tests and existing suite pass.

### Task 2: Implement Approval Queue Durability And Resolution

**Files:**
- Modify: `crates/sharo-daemon/src/store.rs`
- Modify: `crates/sharo-core/src/protocol.rs`
- Test: `crates/sharo-daemon/tests/approval_flow.rs`

**Preconditions**
- Policy `require_approval` path is implemented.

**Invariants**
- Approval resolution is idempotent by `approval_id`.

**Postconditions**
- Pending approvals are listable and resolvable with explicit state transitions.

**Tests (must exist before implementation)**

Unit:
- `approval_resolution_idempotent_by_approval_id`

Property:
- `approval_expiry_does_not_silently_allow`

Integration:
- `approval_required_step_waits_then_executes_after_approve`

**Red Phase (required before code changes)**

Command: `cargo test --package sharo-daemon approval_flow -- --nocapture`
Expected: fails before approval protocol/store is implemented.

**Implementation Steps**

1. Persist approvals with status and expiry fields.
2. Add list and resolve operations in daemon protocol.

**Green Phase (required)**

Command: `cargo test --workspace`
Expected: approval tests and existing suite pass.

### Task 3: Add CLI Approval Commands And Blocking Visibility

**Files:**
- Modify: `crates/sharo-cli/src/main.rs`
- Test: `crates/sharo-cli/tests/approval_cli.rs`

**Preconditions**
- Approval operations are available in daemon protocol.

**Invariants**
- Blocked and awaiting states expose explicit reason text.

**Postconditions**
- CLI supports `approval list` and `approval resolve` with stable output.

**Tests (must exist before implementation)**

Unit:
- `approval_cli_parsing`

Property:
- `approval_cli_resolve_is_idempotent_on_replay`

Integration:
- `scenario_b_cli_blocked_and_approval_resolution`

**Red Phase (required before code changes)**

Command: `cargo test --package sharo-cli approval_cli -- --nocapture`
Expected: fails before approval CLI commands exist.

**Implementation Steps**

1. Add approval command group and handlers.
2. Add blocked reason and policy summary rendering to task reads.

**Green Phase (required)**

Command: `cargo test --workspace`
Expected: workspace tests pass.

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated
