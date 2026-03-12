# Hazel Inspection Tooling Design

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Invariant, and Integration checks.
> - Use Property-based tests only when a generative framework is actually used.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

Updated: 2026-03-12
Status: completed
Owner: runtime
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-HAZEL-INSPECTION-DESIGN-001, TASK-HAZEL-INSPECTION-PLAN-001, TASK-HAZEL-INSPECTION-SPEC-001

## Goal

Define the daemon/TUI-first architecture for Hazel inspection and safe control tooling.

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. This design's accepted requirements and invariants.
4. Follow-on implementation planning derived from this design.

## Execution Mode

- Mode: plan-only
- Default: this design note constrains implementation work and feeds the implementation plan.

## Output Contract

- Keep the daemon execution model unchanged.
- Keep Hazel inspection/control actions daemon-owned and auditable.
- Preserve Hazel deterministic validation and proposal-producing sleep-job constraints.

## Architecture

Hazel inspection tooling is a daemon control-plane subsystem. The daemon exposes typed Hazel status, inspection, and safe control APIs; the TUI and CLI act only as clients over those APIs. Hazel core logic remains in Hazel crates, while the daemon owns request validation, durable operator-visible records, and control-plane response shaping.

## Accepted Requirements

1. The daemon is the only Hazel authority exposed to operator surfaces.
2. The initial slice includes both inspection and safe control actions.
3. Retrieval preview is derived and non-mutating.
4. Proposal validation and submission remain explicit and validator-gated.
5. Sleep-job controls remain proposal-producing only.
6. TUI Hazel views are derived daemon-backed screens, not local Hazel readers.
7. Hazel control-plane state remains distinct from task/chat/MCP subsystems.

## Boundaries

- In scope:
  - daemon Hazel protocol/control-plane surfaces
  - durable exact records for preview/validation/submission/job lifecycle
  - TUI Hazel screen
  - CLI Hazel command surface over daemon APIs
- Out of scope:
  - local direct Hazel storage access from clients
  - free-form memory editing
  - opaque direct canonical-memory mutation
  - generalized extraction pipelines

## State Rules

1. `sharo-daemon` owns the canonical Hazel control-plane records exposed to clients.
2. Hazel preview/validation/submission/job records are exact operator-visible records.
3. TUI and CLI cache only derived presentation state.
4. Failed Hazel actions must remain inspectable without partial hidden side effects.

## UX Contract

- Add a dedicated Hazel inspection screen in `sharo-tui`.
- Add slash-command and CLI entry points for Hazel status, listing, preview, validation, submission, and job inspection/control.
- Keep the first slice browse-first with explicit action triggers rather than raw record editing.
- Ensure every Hazel action returns enough structured detail for operator triage.

## Testing Focus

- protocol response bounding and shape validation
- deterministic ordering for Hazel list/get surfaces
- preview non-mutation behavior
- submission and validation record durability
- sleep-job proposal-only behavior
- TUI Hazel screen rendering and command dispatch

### Task 1: Record the approved Hazel inspection/control architecture

**Files:**

- Create: `docs/specs/hazel-inspection-tooling.md`
- Create: `docs/plans/2026-03-12-hazel-inspection-tooling-design.md`
- Create: `docs/plans/2026-03-12-hazel-inspection-tooling-plan.md`
- Modify: `docs/tasks/tasks.csv`

**Preconditions**

- Hazel structured-memory spec remains active.
- The approved direction is daemon/TUI-first with safe control actions included.

**Invariants**

- Daemon authority is preserved.
- Hazel deterministic validation and proposal-only sleep constraints are preserved.

**Postconditions**

- The Hazel inspection/control architecture is documented and task-backed.

**Tests (must exist before implementation)**

Unit:
- `docs_define_daemon_owned_hazel_control_plane`

Invariant:
- `docs_preserve_sleep_jobs_as_proposal_producing_only`

Integration:
- `tasks_registry_references_hazel_inspection_artifacts`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `scripts/check-tasks-sync.sh --changed`
Expected: fails until task-registry rows are added for the new artifacts.

**Implementation Steps**

1. Record daemon-owned Hazel inspection/control boundaries.
2. Record TUI/CLI client responsibilities.
3. Record safe-control-action scope and invariants.
4. Add task-registry references.

**Green Phase (required)**

Command: `scripts/doc-lint.sh --changed --strict-new && scripts/check-tasks-registry.sh && scripts/check-tasks-sync.sh --changed`
Expected: docs and task checks pass.

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Docs/task checks passing

## Task Update Contract

- New Hazel operator requirements must map into protocol, daemon records, or client surfaces before implementation continues.
- No follow-on plan may silently broaden safe control actions into opaque direct writes.

## Completion Gate

- This design is complete only when the approved daemon authority, safe-action boundaries, client UX shape, and verification focus are documented and task-backed.

## Model Compatibility Notes

- "Daemon/TUI-first" means TUI and CLI consume daemon APIs, not that Hazel logic moves into clients.
- "Safe control actions" means validator-gated bounded operations only.
