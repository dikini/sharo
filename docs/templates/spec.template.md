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

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. This spec's task contracts and invariants.
4. In-task updates recorded explicitly in this document.

## Output Contract

- Preserve exact section headings in this template unless intentionally revised.
- Keep claims concrete and tied to observable evidence.
- Avoid introducing unstated requirements or hidden assumptions.

## Evidence / Verification Contract

- Every completion claim must cite verification commands/results in `## Verification`.
- Conflicting evidence must be called out explicitly before task closure.
- If verification cannot run, record why and the residual risk.

## Model Compatibility Notes

- XML-style delimiter blocks (e.g. `<context>`, `<constraints>`) are optional structure aids.
- Critical constraints must also be restated in plain language.
- This fallback is required for cross-model robustness (including GPT-5.3 behavior).

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
