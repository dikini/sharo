# Authenticated Provider Base URL Hardening

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Define tests before implementation work.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Property, and Integration tests.
> - Run `scripts/doc-lint.sh` before commit.

Updated: 2026-03-06
Status: active
Owner: codex
Template-Profile: tdd-strict-v1
Task-Registry-Refs: TASK-AUTH-BASE-URL-HARDENING-SPEC-001, TASK-AUTH-BASE-URL-HARDENING-PLAN-001

## Purpose

Prevent authenticated provider configurations from sending bearer tokens to insecure or unintended base URLs.

## Scope

### In Scope

- validation of authenticated provider `base_url`
- explicit rejection of non-HTTPS remote authenticated endpoints
- local test exceptions only when clearly loopback

### Out of Scope

- certificate pinning
- provider allowlists beyond scheme and loopback handling
- retry behavior

## Interfaces / Contracts

- Authenticated OpenAI-compatible provider configs must use HTTPS unless the endpoint is loopback.
- Invalid authenticated base URLs fail fast during config validation or connector execution.

## Invariants

- Tokens are never attached to insecure non-loopback HTTP endpoints.
- Existing unauthenticated local-test scenarios remain possible.

## Task Contracts

### Task 1: Enforce secure authenticated base URLs

**Preconditions**

- Current provider config and connector tests pass.

**Invariants**

- Auth failures and transport errors remain separately classified.
- Lower-case machine-readable error strings remain intact.

**Postconditions**

- Authenticated insecure remote URLs are rejected before any request is sent.

**Tests (must exist before implementation)**

Unit:
- `authenticated_http_base_url_is_rejected`

Property:
- `authenticated_non_https_non_loopback_urls_are_never_accepted`

Integration:
- `authenticated_loopback_http_base_url_remains_allowed_for_local_tests`

## Verification

- `cargo test -p sharo-core --test reasoning_connector_tests -- --nocapture`
- `cargo test -p sharo-daemon --test scenario_a -- --nocapture`

## Risks and Failure Modes

- Accidental token exfiltration to cleartext or internal HTTP endpoints
- Over-restricting local test harnesses

## References

- `crates/sharo-core/src/model_connectors.rs`
- `crates/sharo-daemon/src/kernel.rs`
