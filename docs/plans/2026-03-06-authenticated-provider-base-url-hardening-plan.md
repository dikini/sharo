# Authenticated Provider Base URL Hardening Implementation Plan

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Property, and Integration tests.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: reject authenticated provider configurations that would send bearer tokens to insecure remote endpoints.
Architecture: validate authenticated `base_url` values explicitly and allow only HTTPS or loopback HTTP for local test harnesses. Keep the change narrow so provider error classification and existing local smoke scenarios remain intact.
Tech Stack: Rust 2024, reqwest/url parsing, core connector tests, daemon scenario tests.
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-AUTH-BASE-URL-HARDENING-SPEC-001, TASK-AUTH-BASE-URL-HARDENING-PLAN-001

---

### Task 1: Add failing security coverage for authenticated base URLs

**Files:**

- Modify: `crates/sharo-daemon/src/kernel.rs`
- Modify: `crates/sharo-core/tests/reasoning_connector_tests.rs`

**Preconditions**

- Current provider tests pass.

**Invariants**

- Test cases distinguish secure remote URLs from local loopback fixtures.
- Existing auth-failure semantics remain intact.

**Postconditions**

- There is explicit failing coverage for authenticated insecure remote URLs.

**Tests (must exist before implementation)**

Unit:
- `authenticated_http_base_url_is_rejected`

Property:
- `authenticated_non_https_non_loopback_urls_are_never_accepted`

Integration:
- `authenticated_loopback_http_base_url_remains_allowed_for_local_tests`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-core --test reasoning_connector_tests -- --nocapture`
Expected: new security coverage fails before validation is added.

**Implementation Steps**

1. Add config or connector validation for authenticated URLs.
2. Reject non-HTTPS authenticated remote endpoints with an explicit invalid-config error.
3. Keep loopback HTTP accepted for deterministic local test servers.

**Green Phase (required)**

Command: `cargo test -p sharo-core --test reasoning_connector_tests -- --nocapture`
Expected: security tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/sharo-daemon/src/kernel.rs`, `crates/sharo-core/src/model_connectors.rs`, related tests
Re-run: `cargo clippy -p sharo-core --all-targets -- -D warnings`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated
