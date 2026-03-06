# Provider Error Classification Implementation Plan

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Property, and Integration tests.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: classify provider transport and HTTP failures into retry-correct connector errors instead of collapsing them into `InvalidRequest`.
Architecture: make connector error mapping explicit and test-driven, ideally through a small classification helper that maps transport failures and status codes to connector error variants. The plan follows Rust-skill guidance around custom error types, concise lower-case messages, and descriptive tests.
Tech Stack: Rust 2024, `reqwest`, `serde_json`, core reasoning connector tests.
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-PROVIDER-ERROR-PLAN-001, TASK-PROVIDER-ERROR-SPEC-001

---

### Task 1: Add failing connector classification tests

**Files:**

- Modify: `crates/sharo-core/src/model_connectors.rs`
- Modify: `crates/sharo-core/tests/reasoning_connector_tests.rs`

**Preconditions**

- Existing connector tests pass.

**Invariants**

- Test names stay aligned with observable provider outcomes.
- Coverage distinguishes retryable from terminal classes.

**Postconditions**

- New tests fail against the current misclassification behavior.

**Tests (must exist before implementation)**

Unit:
- `http_500_maps_to_unavailable`
- `http_408_maps_to_timeout`
- `http_429_maps_to_rate_limit`
- `http_402_maps_to_quota`
- `http_400_maps_to_invalid_request`

Property:
- `non_success_statuses_never_default_retryable_codes_to_invalid_request`

Integration:
- `reasoning_engine_surfaces_retryable_provider_failure_without_task_success`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-core --test reasoning_connector_tests -- --nocapture`
Expected: the new classification tests fail before the mapping fix.

**Implementation Steps**

1. Add status-specific connector tests near the adapter.
2. Add one reasoning-level assertion that retryable failures do not look like successful task planning.
3. Keep the new tests focused on classification, not retry-loop behavior.

**Green Phase (required)**

Command: `cargo test -p sharo-core --test reasoning_connector_tests -- --nocapture`
Expected: new and existing connector tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/sharo-core/src/model_connectors.rs`, connector tests
Re-run: `cargo test -p sharo-core`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated

### Task 2: Extract explicit status and transport mapping

**Files:**

- Modify: `crates/sharo-core/src/model_connectors.rs`
- Modify: `crates/sharo-core/src/model_connector.rs`

**Preconditions**

- Task 1 tests are failing.

**Invariants**

- Auth, timeout, availability, protocol, and invalid-request classes remain distinct.
- Message format stays machine-parseable and lower-case.

**Postconditions**

- Retryable statuses and transport errors map to the right connector variants.

**Tests (must exist before implementation)**

Unit:
- `http_500_maps_to_unavailable`
- `http_408_maps_to_timeout`
- `http_429_maps_to_rate_limit`
- `http_402_maps_to_quota`
- `http_400_maps_to_invalid_request`

Property:
- `non_success_statuses_never_default_retryable_codes_to_invalid_request`

Integration:
- `reasoning_engine_surfaces_retryable_provider_failure_without_task_success`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-core model_connectors::tests -- --nocapture`
Expected: classification tests fail until the explicit mapping helper exists.

**Implementation Steps**

1. Introduce a helper that maps `reqwest::StatusCode` to `ConnectorError`.
2. Keep transport error mapping explicit for timeout/connect/other cases.
3. Expand `ConnectorError` only if current variants cannot represent the required matrix cleanly.
4. Re-check downstream formatting so new variants remain visible in daemon/kernel errors.

**Green Phase (required)**

Command: `cargo test -p sharo-core model_connectors::tests -- --nocapture`
Expected: adapter tests pass with explicit classification.

**Refactor Phase (optional but controlled)**

Allowed scope: connector error types and HTTP adapter only
Re-run: `cargo test -p sharo-core --test reasoning_connector_tests`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated

### Task 3: Re-verify daemon/kernel behavior with corrected error classes

**Files:**

- Modify: `crates/sharo-daemon/tests/scenario_a.rs`
- Modify: `crates/sharo-core/tests/reasoning_connector_tests.rs`

**Preconditions**

- Explicit classification is implemented.

**Invariants**

- No provider failure class is misreported as success.
- Existing auth-failure expectations remain intact.

**Postconditions**

- Runtime tests reflect the retryability distinction without altering task-success semantics.

**Tests (must exist before implementation)**

Unit:
- `http_500_maps_to_unavailable`

Property:
- `non_success_statuses_never_default_retryable_codes_to_invalid_request`

Integration:
- `reasoning_engine_surfaces_retryable_provider_failure_without_task_success`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-daemon --test scenario_a -- --nocapture`
Expected: either fails for the new scenario or stays green after the new assertion is added.

**Implementation Steps**

1. Add or adjust one daemon-level scenario to pin expected behavior for transient provider failure.
2. Verify connector classification still feeds kernel error strings predictably.
3. Keep the integration surface narrow and evidence-based.

**Green Phase (required)**

Command: `cargo test -p sharo-core --test reasoning_connector_tests && cargo test -p sharo-daemon --test scenario_a -- --nocapture`
Expected: core and daemon integration tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: test wording and helper extraction only
Re-run: `scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated
