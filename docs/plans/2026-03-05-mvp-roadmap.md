# MVP Roadmap Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: define and track a scenario-driven committable roadmap from current runtime state to MVP-as-specified.
Architecture: this roadmap is itself a strict plan artifact that defines ordered slices and their delivery gates. Execution happens in per-slice plan files to keep documentation manageable and merge-safe.
Tech Stack: markdown, bash policy checks, task registry csv.
Template-Profile: tdd-strict-v1

---

### Task 1: Define Ordered MVP Slices And Exit Gates

**Files:**
- Modify: `docs/plans/2026-03-05-mvp-roadmap.md`
- Reference: `docs/specs/mvp.md`

**Preconditions**
- `docs/specs/mvp.md` is active and contains mandatory scenarios and verification matrix.

**Invariants**
- Work is split into committable slices; no single monolithic implementation plan is introduced.

**Postconditions**
- Roadmap defines ordered slices, dependencies, and completion gates.

**Tests (must exist before implementation)**

Unit:
- `roadmap_lists_all_slice_ids`

Property:
- `slice_order_is_strictly_monotonic`

Integration:
- `roadmap_references_existing_slice_plan_files`

**Red Phase (required before code changes)**

Command: `scripts/doc-lint.sh --path docs/plans/2026-03-05-mvp-roadmap.md --strict-new`
Expected: fails while required strict sections or links are incomplete.

**Implementation Steps**

1. Define slices:
   - `Slice 000` roadmap and tracking bootstrap
   - `Slice 001` Scenario A end-to-end read success
   - `Slice 002` Scenario B policy and approvals
   - `Slice 003` Scenario C overlap and coordination
   - `Slice 004` protocol and CLI completion
   - `Slice 005` verification matrix closure and hardening
2. Define per-slice completion gate rule:
   - tests for slice pass
   - task registry status updated
   - changelog updated
3. Link per-slice plan artifacts:
   - [Slice 000 Plan](./2026-03-05-mvp-slice-000-roadmap-tracking-plan.md)
   - [Slice 001 Plan](./2026-03-05-mvp-slice-001-scenario-a-plan.md)
   - [Slice 002 Plan](./2026-03-05-mvp-slice-002-scenario-b-plan.md)
   - [Slice 003 Plan](./2026-03-05-mvp-slice-003-scenario-c-plan.md)
   - [Slice 004 Plan](./2026-03-05-mvp-slice-004-protocol-cli-plan.md)
   - [Slice 005 Plan](./2026-03-05-mvp-slice-005-verification-hardening-plan.md)
4. Define final roadmap exit criteria:
   - mandatory scenarios A B C pass
   - required protocol and CLI surface implemented
   - verification matrix rows are evidence-backed
   - fast-feedback and CI checks pass

**Green Phase (required)**

Command: `scripts/doc-lint.sh --changed --strict-new && scripts/check-tasks-sync.sh --changed`
Expected: doc lint and task sync checks pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `docs/plans/2026-03-05-mvp-roadmap.md`
Re-run: `scripts/doc-lint.sh --path docs/plans/2026-03-05-mvp-roadmap.md --strict-new`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated
