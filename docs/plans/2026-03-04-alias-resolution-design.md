# Alias Resolution Design

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Property, and Integration tests.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

Updated: 2026-03-04
Status: active
Owner: sharo
Template-Profile: tdd-strict-v1

## Goal

Provide stable, human-friendly canonical aliases (for example `@spec:mvp`) that resolve to repo paths without introducing line-anchor drift.

## Architecture

Version 1 is file-level only: aliases map to canonical repo-relative file paths in a centralized TOML registry. The resolver script is shell-only, deterministic, and fails closed for unknown aliases and invalid targets. Line-level or semantic selectors are intentionally deferred.

## Task Contracts

### Task 1: Confirm V1 Alias Design Constraints

**Preconditions**

- [x] Decision is approved to avoid line-anchor selectors in v1.

**Invariants**

- [x] Alias registry remains canonical and repo-relative.
- [x] Resolver behavior remains deterministic and fail-closed.

**Postconditions**

- [x] Design captures file-only aliases, central registry, and extension path.

**Tests (must exist before implementation)**

Unit:
- [x] `design_declares_file_only_alias_resolution`

Property:
- [x] `design_avoids_non_deterministic_selectors`

Integration:
- [x] `implementation_plan_references_design_constraints`

**Red Phase (required before code changes)**

Command: `test -f docs/aliases.toml`
Expected: file not found before implementation.

**Implementation Steps**

1. Define v1 scope and constraints.
2. Record in-scope and out-of-scope behavior.
3. Link design constraints into implementation plan.

**Green Phase (required)**

Command: `rg -n \"file-level only|Line-level or semantic selectors are intentionally deferred\" docs/plans/2026-03-04-alias-resolution-design.md`
Expected: constraints present.

**Refactor Phase (optional but controlled)**

Allowed scope: wording only.
Re-run: `scripts/doc-lint.sh --path docs/plans/2026-03-04-alias-resolution-design.md --strict-new`

**Completion Evidence**

- [x] Preconditions satisfied
- [x] Invariants preserved
- [x] Postconditions met
- [x] Unit, property, and integration tests passing
- [x] CHANGELOG.md updated
