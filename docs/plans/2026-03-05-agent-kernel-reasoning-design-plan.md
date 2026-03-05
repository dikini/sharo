# Agent Kernel Reasoning Design Implementation Plan

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Property, and Integration tests.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: define a stable kernel/reasoning/model-connector architecture for MVP-preserving evolution and multi-provider expansion.
Architecture: use ports-and-adapters boundaries in `sharo-core` and keep daemon IPC/CLI contracts stable while execution logic moves behind kernel interfaces. Model access uses a unified turn contract, with provider-specific adapters mapped to one error and capability model. Research choices and dependency decisions are captured explicitly so future implementation does not re-open design questions.
Tech Stack: Rust 2024, `serde`, `reqwest`, existing daemon store + protocol surfaces, maintained provider APIs.
Template-Profile: tdd-strict-v1

---

### Task 1: Lock Interface Design and Contracts

Task-ID: TASK-KERNEL-DESIGN-001

**Files:**

- Modify: `docs/plans/2026-03-05-agent-kernel-reasoning-design-plan.md`
- Modify: `docs/specs/mvp.md`
- Test: `scripts/doc-lint.sh`

**Preconditions**

- `docs/specs/mvp.md` is current source of truth for runtime invariants.
- Existing scenario A/B/C behavior is implemented and passing.

**Invariants**

- daemon IPC request/response contracts remain unchanged by design decisions in this task.
- behavior-equivalence-first policy is explicit and preserved.
- pre-1.0 backward-compatibility posture remains unchanged.

**Postconditions**

- contract definitions for `KernelPort`, `ReasoningEnginePort`, and `ModelConnectorPort` are decision-complete.
- MVP trace/artifact behavior preservation boundaries are explicit.

**Tests (must exist before implementation)**

Unit:
- `design_contains_kernel_reasoning_connector_contracts`

Property:
- `design_keeps_ipc_surface_stable_while_internal_ports_evolve`

Integration:
- `design_references_mvp_invariants_for_behavior_equivalence`

**Red Phase (required before code changes)**

Command: `scripts/doc-lint.sh --path docs/plans/2026-03-05-agent-kernel-reasoning-design-plan.md --strict-new`
Expected: fails before placeholders are replaced and strict sections are complete.

**Implementation Steps**

1. Specify core interface boundaries:
   - `KernelPort` submit/approval operations.
   - `ReasoningEnginePort` plan/evaluate operations.
   - `ModelConnectorPort` normalized turn contract.
2. Define connector profile model:
   - provider id, model id, base URL, auth source, timeout, retries, capability flags.
3. Define error taxonomy:
   - auth, rate-limit/quota, invalid request, timeout, unavailable, protocol mismatch, internal.
4. Record behavior-equivalence envelope:
   - existing scenario A/B/C event/artifact semantics remain stable in extraction phase.
5. Link decisions to MVP spec constraints and non-goals.

### Task 2: Capture Research and Dependency Policy

Task-ID: TASK-KERNEL-DESIGN-002

**Files:**

- Modify: `docs/plans/2026-03-05-agent-kernel-reasoning-design-plan.md`
- Modify: `docs/tasks/tasks.csv`
- Test: `scripts/check-tasks-registry.sh`, `scripts/check-tasks-sync.sh --changed`

**Preconditions**

- local research inputs are available (`~/Projects/codex/codex-rs`, provider docs, crates metadata).

**Invariants**

- dependency choices prioritize maintained libraries with compatible licenses.
- no framework takeover that breaks MVP trace/approval semantics.

**Postconditions**

- research-backed choices are documented:
  - own thin kernel + selective libraries.
  - OpenAI-compatible connector path for OpenAI/OpenRouter/compatible third parties.
  - local connector path for Ollama.
- task registry tracks the design artifact.

**Tests (must exist before implementation)**

Unit:
- `design_lists_selected_and_rejected_libraries_with_reason`

Property:
- `dependency_policy_requires_maintenance_and_license_screening`

Integration:
- `tasks_registry_references_design_task_ids`

**Red Phase (required before code changes)**

Command: `scripts/check-tasks-sync.sh --changed`
Expected: fails until `docs/tasks/tasks.csv` is updated for new docs/spec references.

**Implementation Steps**

1. Document codex-rs learnings to preserve:
   - provider registry pattern.
   - session-scoped client with per-turn request context.
   - local adapter readiness/probing behavior.
2. Record provider docs and contract expectations:
   - OpenAI Responses API.
   - OpenRouter OpenAI-compatible API.
   - Ollama OpenAI compatibility.
3. Record crate selection posture:
   - allow permissive + MPL licenses only.
   - avoid unmaintained/low-signal crates for core runtime paths.
4. Update task registry with explicit design task rows.

**Green Phase (required)**

Command: `scripts/doc-lint.sh --changed --strict-new && scripts/check-tasks-registry.sh && scripts/check-tasks-sync.sh --changed`
Expected: docs lint and task sync/registry checks pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `docs/plans/2026-03-05-agent-kernel-reasoning-design-plan.md`, `docs/tasks/tasks.csv`
Re-run: `scripts/doc-lint.sh --changed --strict-new && scripts/check-tasks-registry.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated
