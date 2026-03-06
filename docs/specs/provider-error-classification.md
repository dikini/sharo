# Provider Error Classification

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

Task-Registry-Refs: TASK-PROVIDER-ERROR-SPEC-001, TASK-PROVIDER-ERROR-PLAN-001

## Purpose

Classify provider transport and HTTP response failures into connector error kinds that preserve retry semantics and operator diagnostics.

## Scope

### In Scope

- OpenAI-compatible and Ollama connector HTTP status mapping.
- Error messages and tests for retryable versus terminal failure classes.
- Optional connector error-type expansion if required for quota/backoff fidelity.

### Out of Scope

- Retry-loop implementation itself.
- Non-HTTP providers.
- CLI or daemon protocol changes unrelated to connector errors.

## Core Terms

- `Retryable Failure`: a provider error that should surface as `Unavailable`, `Timeout`, `RateLimit`, or `Quota`.
- `Terminal Client Failure`: a malformed request or unsupported feature that should surface as `InvalidRequest`.
- `Classification Matrix`: the status-code to connector-error mapping contract.

## Interfaces / Contracts

- HTTP `5xx` must not be surfaced as `InvalidRequest`.
- Known retryable classes keep machine-parseable, lower-case messages.
- Connector error mapping must remain provider-agnostic at the trait boundary.

## Invariants

- Authentication failures remain distinct from generic availability failures.
- Retryable transport classes do not collapse into terminal client errors.
- Parsing failures remain `ProtocolMismatch`.

## Task Contracts

### Task 1: Normalize HTTP and Transport Error Mapping

**Preconditions**

- `OpenAiCompatibleConnector` remains the HTTP baseline adapter.

**Invariants**

- Status mapping is table-driven or equivalently explicit.
- Error text stays concise and machine-parseable.

**Postconditions**

- Provider 5xx, timeout, rate-limit, and quota signals map to retry-appropriate connector errors.

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

## Scenarios

- S1: provider returns `500`; runtime reports temporary unavailability.
- S2: provider returns `429`; runtime reports rate limit with no false success task.
- S3: provider returns `400`; runtime reports invalid request.

## Verification

- `cargo test -p sharo-core --test reasoning_connector_tests -- --nocapture`
- `cargo test -p sharo-core model_connectors::tests::http_500_maps_to_unavailable -- --nocapture`
- `scripts/check-fast-feedback.sh`

## Risks and Failure Modes

- Overfitting to a single provider’s status semantics could make the abstraction less portable.
- Expanding the enum without updating downstream formatting can hide new failure detail.

## Open Questions

- Should `ConnectorError` add a structured `status_code` field in a follow-up change?

## References

- [docs/plans/2026-03-05-agent-kernel-reasoning-implementation-plan.md](/home/dikini/Projects/sharo/docs/plans/2026-03-05-agent-kernel-reasoning-implementation-plan.md)
- Rust skills: `err-custom-type`, `err-lowercase-msg`, `test-descriptive-names`
