# Store Transactional Persistence

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Define tests before implementation work.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Property, and Integration tests.
> - Run `scripts/doc-lint.sh` before commit.

Updated: 2026-03-06
Status: active
Owner: runtime
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-STORE-TRANSACTIONAL-SPEC-001, TASK-STORE-TRANSACTIONAL-PLAN-001

## Purpose

Prevent in-memory store state from diverging from disk state when persistence fails.

## Scope

### In Scope

- Session, task, approval, binding, and idempotency mutations in `Store`.
- Rollback or commit-style mutation flow around `save()`.
- Tests for save-failure behavior and retry correctness.

### Out of Scope

- Migrating the store to a database.
- Changing persisted JSON schema versioning policy.
- Adding remote replication or backup behavior.

## Core Terms

- `Transactional Mutation`: a store update that becomes visible only after persistence succeeds.
- `Save Failure`: any error returned from serialization, temp-file creation, write, sync, rename, or chmod.
- `Replay Consistency`: retrying the same client intent after a failed save must not observe partial prior mutation.

## Interfaces / Contracts

- Mutating `Store` APIs must either fully apply and persist or leave in-memory state unchanged.
- Failed persistence must not consume IDs, create ghost sessions/tasks, or poison idempotency replay tables.
- Existing success-path response payloads remain unchanged.

## Invariants

- In-memory and on-disk states are equivalent after every successful mutating operation.
- A failed save leaves state observationally identical to the pre-call state.
- Idempotency replay tables track only committed outcomes.

## Task Contracts

### Task 1: Make Store Mutations Commit-Or-Rollback

**Preconditions**

- `Store` persists through `save()` using atomic rename.

**Invariants**

- Mutation staging is isolated from the live `Store.state` until commit.
- Error return paths preserve pre-call state exactly.

**Postconditions**

- All mutating store APIs are transactional with respect to persistence failure.

**Tests (must exist before implementation)**

Unit:
- `register_session_rolls_back_when_save_fails`
- `submit_task_rolls_back_when_save_fails`
- `resolve_approval_rolls_back_when_save_fails`

Property:
- `failed_store_mutation_preserves_pre_call_state`

Integration:
- `idempotent_retry_after_save_failure_creates_one_committed_task`

## Scenarios

- S1: store parent path is invalid; `register_session` returns error and the next successful call still creates `session-000001`.
- S2: task submit save fails after task construction; retry does not replay a ghost task.
- S3: approval resolution save fails; task state and approval state remain pending.

## Verification

- `cargo test -p sharo-daemon store::tests::register_session_rolls_back_when_save_fails -- --nocapture`
- `cargo test -p sharo-daemon store::tests::submit_task_rolls_back_when_save_fails -- --nocapture`
- `cargo test -p sharo-daemon --test scenario_a idempotent_retry_after_save_failure_creates_one_committed_task -- --nocapture`
- `scripts/check-fast-feedback.sh`

## Risks and Failure Modes

- Naive full-state cloning could increase memory churn; avoid unnecessary clones where a staged state can be reused.
- Partial helper extraction may leave one mutating path non-transactional.

## Open Questions

- Should commit-style staging be implemented as cloned state replacement or typed mutation commands with undo?

## References

- [docs/plans/2026-03-05-restart-trace-continuity-hardening-plan.md](/home/dikini/Projects/sharo/docs/plans/2026-03-05-restart-trace-continuity-hardening-plan.md)
- Rust skills: `err-result-over-panic`, `mem-reuse-collections`, `test-fixture-raii`
