# <Spec Title>

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Define tests before implementation work.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Invariant, and Integration checks.
> - Use Property-based tests only when a generative framework is actually used.
> - Run `scripts/doc-lint.sh` before commit.

Updated: <YYYY-MM-DD>
Status: draft | active | deprecated
Owner: <team/person>
Template-Profile: tdd-strict-v1

## Purpose

## Scope

### In Scope

### Out of Scope

## Core Terms

## Interfaces / Contracts

## Invariants

## Task Contracts

### Task N: <Task Name>

**Preconditions**

- <required state or dependency>

**Invariants**

- <must remain true during and after task>

**Postconditions**

- <observable completion condition>

**Tests (must exist before implementation)**

Unit:
- <test id or test name>

Invariant:
- <test id or test name>

Integration:
- <test id or test name>

Property-based (optional):
- <test id or test name; only when using generative tooling such as proptest/quickcheck>

## Scenarios

## Verification

## Risks and Failure Modes

## Open Questions

## References
