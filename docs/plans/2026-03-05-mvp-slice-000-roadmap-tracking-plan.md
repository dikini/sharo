# MVP Slice 000 Roadmap And Tracking Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: establish MVP delivery roadmap and deterministic slice tracking.
Architecture: document-only slice that introduces roadmap and per-slice plan files, then registers all slices in the task registry. This slice does not change runtime behavior.
Tech Stack: markdown, bash policy scripts, task registry CSV.
Template-Profile: tdd-strict-v1

---

### Task 1: Add Roadmap And Slice Plans

**Files:**
- Create: `docs/plans/2026-03-05-mvp-roadmap.md`
- Create: `docs/plans/2026-03-05-mvp-slice-00*-*.md`

**Preconditions**
- `docs/specs/mvp.md` is active and authoritative.

**Invariants**
- Plan docs remain split by slice; no monolithic implementation file.

**Postconditions**
- One roadmap doc and per-slice plan docs exist and cross-reference each other.

**Tests (must exist before implementation)**

Unit:
- `roadmap_doc_references_all_slice_plans`

Property:
- `slice_plan_filenames_follow_date_prefix_convention`

Integration:
- `doc_lint_accepts_new_plan_docs`

**Red Phase (required before code changes)**

Command: `scripts/doc-lint.sh --path docs/plans/2026-03-05-mvp-roadmap.md --strict-new`
Expected: fails before the files exist.

**Implementation Steps**

1. Add roadmap doc with slice sequence and exit criteria.
2. Add one plan file per slice with task-level breakdown.

**Green Phase (required)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: docs lint passes for new roadmap and slice plan files.

**Refactor Phase (optional but controlled)**

Allowed scope: `docs/plans/*mvp-slice*`, `docs/plans/2026-03-05-mvp-roadmap.md`
Re-run: `scripts/doc-lint.sh --changed --strict-new`

### Task 2: Register Slice Tasks

**Files:**
- Modify: `docs/tasks/tasks.csv`
- Modify: `docs/tasks/README.md`

**Preconditions**
- New slice plan docs exist.

**Invariants**
- Each task id in CSV is referenced in `docs/tasks/README.md`.

**Postconditions**
- All slice tasks are tracked with deterministic status values.

**Tests (must exist before implementation)**

Unit:
- `tasks_registry_rows_have_valid_statuses`

Property:
- `slice_ids_are_unique`

Integration:
- `tasks_registry_source_reference_rule_passes`

**Red Phase (required before code changes)**

Command: `scripts/check-tasks-registry.sh`
Expected: passes before changes and fails if source references are broken during edit.

**Implementation Steps**

1. Add `TASK-MVP-SLICE-000` through `TASK-MVP-SLICE-005` rows.
2. Add matching references and commands in `docs/tasks/README.md`.

**Green Phase (required)**

Command: `scripts/check-tasks-registry.sh && scripts/check-tasks-sync.sh --changed`
Expected: both checks pass with updated registry and docs.

**Refactor Phase (optional but controlled)**

Allowed scope: `docs/tasks/README.md`, `docs/tasks/tasks.csv`
Re-run: `scripts/check-tasks-registry.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated
