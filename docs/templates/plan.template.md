# <Feature> Implementation Plan

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Property, and Integration tests.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: <one sentence>
Architecture: <2-3 sentences>
Tech Stack: <tools/libraries>
Template-Profile: tdd-strict-v1

---

### Task N: <Task Name>

**Files:**

- Create:
- Modify:
- Test:

**Preconditions**

- [ ] <required state or dependency>

**Invariants**

- [ ] <must remain true during and after task>

**Postconditions**

- [ ] <observable completion condition>

**Tests (must exist before implementation)**

Unit:
- [ ] <test id or test name>

Property:
- [ ] <test id or test name>

Integration:
- [ ] <test id or test name>

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

- [ ] Preconditions satisfied
- [ ] Invariants preserved
- [ ] Postconditions met
- [ ] Unit, property, and integration tests passing
- [ ] CHANGELOG.md updated
