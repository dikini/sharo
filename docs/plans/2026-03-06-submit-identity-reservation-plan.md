# Submit Identity Reservation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** allocate unique task and turn identities before reasoning starts so concurrent submits do not share logical trace scope.

**Architecture:** move submit identity allocation from derived read-time hints to an explicit reservation step in the store. Preserve replay handling for duplicate idempotency keys, and keep persisted task IDs aligned with reserved identities.

**Tech Stack:** Rust 2024, daemon store unit tests, daemon scenario tests.

---

Template-Profile: tdd-strict-v1
Task-Registry-Refs: TASK-SUBMIT-IDENTITY-SPEC-001, TASK-SUBMIT-IDENTITY-PLAN-001

### Task 1: Reserve submit identity before reasoning

**Files:**
- Modify: `crates/sharo-daemon/src/store.rs`
- Modify: `crates/sharo-daemon/src/kernel.rs`
- Modify: `crates/sharo-daemon/tests/scenario_a.rs`

**Preconditions**

- Existing concurrent submit tests pass.

**Invariants**

- Replay semantics for duplicate idempotency keys remain unchanged.
- Concurrent non-idempotent submits cannot reuse reserved task or turn identities.

**Postconditions**

- Same-session concurrent preparations receive distinct task and turn identities.
- Reasoning input uses reserved identities rather than derived read-time hints.

**Tests (must exist before implementation)**

Unit:
- `prepare_submit_reserves_unique_hints_under_concurrency`

Property:
- `concurrent_same_session_submits_never_share_turn_or_task_hints`

Integration:
- `parallel_same_session_submits_produce_distinct_trace_scopes`

**Red Phase (required before code changes)**

Run: `cargo test -p sharo-daemon prepare_submit_reserves_unique_hints_under_concurrency -- --exact`
Expected: FAIL because current preparation derives hints from committed state only.

**Implementation Steps**

1. Add failing store coverage for concurrent same-session preparation.
2. Add scenario coverage that proves concurrent submits do not share trace scope.
3. Refactor store preparation to reserve identities atomically.
4. Update submit execution to consume reserved identities through reasoning and commit paths.
5. Re-run focused tests and the daemon crate suite.

**Green Phase (required)**

Run: `cargo test -p sharo-daemon prepare_submit_reserves_unique_hints_under_concurrency -- --exact`
Expected: PASS.

**Completion Evidence**

- Focused red/green test recorded
- `cargo test -p sharo-daemon` passes
- `scripts/check-fast-feedback.sh` passes
- `CHANGELOG.md` updated
