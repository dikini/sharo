# Store Transactional Persistence Implementation Plan

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Property, and Integration tests.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: make store mutations commit only after persistence succeeds so failed writes do not leave ghost state in memory.
Architecture: stage all mutating `Store` operations off the live state, persist the staged state, then swap it into place only on success. The plan leans on Rust-skill guidance around explicit error handling and avoiding hidden partial state transitions.
Tech Stack: Rust 2024, Serde JSON, filesystem persistence, daemon store tests.
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-STORE-TRANSACTIONAL-PLAN-001, TASK-STORE-TRANSACTIONAL-SPEC-001

---

### Task 1: Add failing save-rollback coverage

**Files:**

- Modify: `crates/sharo-daemon/src/store.rs`
- Modify: `crates/sharo-daemon/tests/scenario_a.rs`

**Preconditions**

- Current store tests pass.

**Invariants**

- New failure tests use deterministic, local filesystem fixtures.
- Tests assert state before and after the failed call.

**Postconditions**

- There are failing tests proving save failure currently mutates in-memory state.

**Tests (must exist before implementation)**

Unit:
- `register_session_rolls_back_when_save_fails`
- `submit_task_rolls_back_when_save_fails`
- `resolve_approval_rolls_back_when_save_fails`

Property:
- `failed_store_mutation_preserves_pre_call_state`

Integration:
- `idempotent_retry_after_save_failure_creates_one_committed_task`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-daemon register_session_rolls_back_when_save_fails submit_task_rolls_back_when_save_fails resolve_approval_rolls_back_when_save_fails -- --nocapture`
Expected: the new rollback tests fail against the current mutate-then-save behavior.

**Implementation Steps**

1. Add helper fixtures that force `save()` failure without relying on nondeterministic disk errors.
2. Capture pre-call snapshots and assert exact equality after failure.
3. Add an integration retry case for idempotent submit behavior after failed persistence.

**Green Phase (required)**

Command: `cargo test -p sharo-daemon register_session_rolls_back_when_save_fails submit_task_rolls_back_when_save_fails resolve_approval_rolls_back_when_save_fails -- --nocapture`
Expected: rollback tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/sharo-daemon/src/store.rs`
Re-run: `cargo test -p sharo-daemon store::tests -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated

### Task 2: Introduce staged-state commit helpers

**Files:**

- Modify: `crates/sharo-daemon/src/store.rs`

**Preconditions**

- Rollback tests fail.

**Invariants**

- Live state is replaced only after successful persistence.
- Error paths leave the live state untouched.

**Postconditions**

- `register_session`, submit paths, and approval resolution are transactional.

**Tests (must exist before implementation)**

Unit:
- `register_session_rolls_back_when_save_fails`
- `submit_task_rolls_back_when_save_fails`
- `resolve_approval_rolls_back_when_save_fails`

Property:
- `failed_store_mutation_preserves_pre_call_state`

Integration:
- `idempotent_retry_after_save_failure_creates_one_committed_task`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-daemon store::tests -- --nocapture`
Expected: new rollback tests fail before staged commit exists.

**Implementation Steps**

1. Add a helper that clones or otherwise stages `PersistedState` for mutation.
2. Persist the staged state via a `save_state(&PersistedState)` helper.
3. Replace live state only after `save_state` succeeds.
4. Apply the helper uniformly across all mutating APIs to avoid one-off divergence bugs.

**Green Phase (required)**

Command: `cargo test -p sharo-daemon store::tests -- --nocapture`
Expected: store tests pass with transactional persistence.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/sharo-daemon/src/store.rs`
Re-run: `cargo test -p sharo-daemon -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated

### Task 3: Re-verify idempotency and restart-oriented flows

**Files:**

- Modify: `crates/sharo-daemon/tests/scenario_a.rs`
- Modify: `crates/sharo-daemon/tests/daemon_ipc.rs`

**Preconditions**

- Transactional store changes are green in unit tests.

**Invariants**

- Persisted task, trace, and approval semantics remain unchanged on the success path.
- Idempotency behavior still reflects only committed outcomes.

**Postconditions**

- Regression coverage proves retries and restart-oriented scenarios behave correctly after save failures.

**Tests (must exist before implementation)**

Unit:
- `register_session_rolls_back_when_save_fails`

Property:
- `failed_store_mutation_preserves_pre_call_state`

Integration:
- `idempotent_retry_after_save_failure_creates_one_committed_task`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-daemon --test scenario_a idempotent_retry_after_save_failure_creates_one_committed_task -- --nocapture`
Expected: fails until the scenario is added.

**Implementation Steps**

1. Add an end-to-end daemon scenario that injects one save failure then retries the same logical request.
2. Assert that only one task/session/approval is committed.
3. Re-run restart continuity and approval flows to catch unintended persistence regressions.

**Green Phase (required)**

Command: `cargo test -p sharo-daemon --test scenario_a idempotent_retry_after_save_failure_creates_one_committed_task -- --nocapture`
Expected: new integration coverage passes.

**Refactor Phase (optional but controlled)**

Allowed scope: scenario coverage only
Re-run: `scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated
