# <Feature> Implementation Plan

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Invariant, and Integration checks.
> - Use Property-based tests only when a generative framework is actually used.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: <one sentence>
Architecture: <2-3 sentences>
Tech Stack: <tools/libraries>
Template-Profile: tdd-strict-v1

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. This plan's execution and verification contracts.
4. Explicit plan updates recorded in this file.

## Execution Mode

- Mode: plan-only | execute-with-checkpoints
- Default: execute-with-checkpoints unless user explicitly requests plan-only output.

## Output Contract

- Keep task definitions concrete: exact files, commands, and expected outcomes.
- Use Red/Green checkpoints as hard gates before claiming task completion.
- Record unresolved risks instead of silently skipping checks.

## Task Update Contract

- New instructions must be mapped to affected tasks before continuing execution.
- If priority conflicts exist, apply Instruction Priority and document the resolution.
- Do not silently drop prior accepted requirements.

## Completion Gate

- A task is complete only when Preconditions, Invariants, Postconditions, and Tests are all satisfied.
- Plan completion requires explicit verification evidence and changelog/task-registry alignment.

## Model Compatibility Notes

- XML-style delimiter blocks are optional and treated as plain-text structure.
- Critical constraints must be restated in plain language for model-robust adherence.

---

### Task N: <Task Name>

**Files:**

- Create:
- Modify:
- Test:

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

**Red Phase (required before code changes)**

Command: `<exact command>`
Expected: failing tests for this task only

**Implementation Steps**

1. <minimal change 1>
2. <minimal change 2>

**Green Phase (required)**

Command: `<exact command>`
Expected: all task tests pass

**Refactor Phase (optional but controlled)**

Allowed scope: <files/components>
Re-run: `<exact command>`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated
