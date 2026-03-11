# Chat-First TUI Design

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Invariant, and Integration checks.
> - Use Property-based tests only when a generative framework is actually used.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

Updated: 2026-03-11
Status: active
Owner: runtime
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-TUI-DESIGN-001, TASK-TUI-PLAN-001

## Goal

Define a first interactive TUI surface for Sharo that enables meaningful daemon-backed chat sessions while preserving the current daemon-centric architecture and exact-state runtime model.

## Architecture

Add a new `sharo-tui` crate as a peer surface beside `sharo-cli`, keep the daemon as canonical runtime authority, and render chat as a derived view over session/task/trace/artifact/approval state. Introduce only the minimum control-plane protocol needed for session views, slash commands, skill catalog/activation, MCP status, and runtime diagnostics. Treat skills, MCP servers, approvals, and future capabilities as separate architectural categories.

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. This design's accepted requirements and invariants.
4. Follow-on implementation planning derived from this design.

## Execution Mode

- Mode: plan-only
- Default: this design note constrains implementation work and feeds the implementation plan.

## Output Contract

- Preserve the daemon as canonical runtime authority.
- Keep the scriptable CLI and interactive TUI as separate surfaces.
- Treat chat transcript rendering as a derived view over canonical task, trace, artifact, and approval state.
- Keep skills, MCP servers, runtime approvals, and future capabilities as distinct taxonomies.
- Require property-based and fuzzing coverage for parser, protocol, and discovery surfaces that accept variable or adversarial input.

## Task Update Contract

- New accepted requirements must be mapped into this design and the implementation plan before coding continues.
- TOML-configurable items must stay distinct from session/runtime state.
- No update may collapse skills, MCP servers, and capabilities into one ambiguous feature bucket.

## Completion Gate

- This design is complete only when the approved architecture, UX constraints, configuration boundaries, and verification expectations are documented and linked to an implementation plan.
- Task-registry references and changelog updates must be present before closure.

## Model Compatibility Notes

- Slash commands are operator-visible control actions, not hidden prompt conventions.
- Skills follow Agent Skills progressive disclosure so full skill contents are loaded only when activated or explicitly inspected.
- TUI-visible transcript and session summaries remain derived views and must not become canonical runtime state.
- Session-task and session-view retrieval stay bounded to recent-task windows so chat-first control-plane surfaces cannot turn into unbounded transport payloads.
- Accepted session ids become durable session records even for implicit submit flows so every conversation visible to the daemon remains reachable through the control-plane catalog.

## Task Contracts

### Task 1: Record the approved chat-first TUI architecture and constraints

**Preconditions**

- [x] Current implementation boundaries are understood from `crates/sharo-core`, `crates/sharo-daemon`, and `crates/sharo-cli`.
- [x] Architecture context from `sharo/2026-03-06-architecture-current-state.md`, `sharo/hermes-sharo.md`, and `sharo/ironclaw-sharo.md` has been reviewed.
- [x] User approved chat-first surface, screen switching, slash commands, inline approvals, session switching, Agent Skills conformance, recursive bundled skill discovery, TOML boundary planning, and property/fuzz coverage.

**Invariants**

- [x] The daemon remains the single canonical runtime authority.
- [x] The TUI is a peer surface rather than a runtime replacement.
- [x] The transcript is a derived operator view over canonical state.
- [x] Session/task retrieval for TUI-oriented control-plane APIs remains bounded.
- [x] Accepted session ids are materialized into durable session records before session-oriented retrieval depends on them.
- [x] Slash commands never bypass approval or policy semantics.
- [x] Skills use bounded recursive discovery under configured roots with deterministic precedence.
- [x] Project scope overrides user scope for the same relative skill id.
- [x] MCP servers, skills, runtime approvals, and future capabilities remain separate subsystems.

**Postconditions**

- [x] The TUI is defined as chat-first with switchable `Chat`, `Sessions`, `Approvals`, `Trace/Artifacts`, and `Settings` screens.
- [x] Slash-command UX is part of the core interaction model.
- [x] Inline approval surfacing for the active conversation is required.
- [x] Fast switching between multiple active sessions is required.
- [x] Skills conform to Agent Skills and use bounded recursive discovery from `.agents/skills/` and `~/.agents/skills/`.
- [x] Skill identity is the stable relative path under the configured root.
- [x] TOML-configurable items are limited to genuine configuration such as skills roots, depth, trust policy, model profiles, and MCP server definitions, while MCP enable/disable toggles remain persisted runtime state.
- [x] Property-based and fuzzing verification are mandatory for parser, protocol, and discovery surfaces where applicable.
- [x] Session-view and session-task retrieval semantics explicitly document bounded recent-task windows and implicit-session materialization.

**Tests (must exist before implementation)**

Unit:
- [x] `design_defines_chat_first_screen_model`
- [x] `design_declares_agent_skills_discovery_rules`

Invariant:
- [x] `design_preserves_daemon_as_canonical_runtime`
- [x] `design_keeps_transcript_as_derived_view`

Integration:
- [x] `implementation_plan_references_tui_design_constraints`
- [x] `task_registry_references_tui_design_artifact`

Property-based (optional):
- [x] `design_requires_property_and_fuzz_coverage_for_variable_input_surfaces`

**Red Phase (required before code changes)**

Command: `scripts/doc-lint.sh --path docs/plans/2026-03-11-chat-first-tui-design.md --strict-new`
Expected: fails before the strict sections and accepted constraints are fully recorded.

**Implementation Steps**

1. Record the approved chat-first surface and switchable screen model.
2. Record the slash-command, inline-approval, and session-switching requirements.
3. Record Agent Skills conformance, bounded recursive discovery, and relative skill-id rules.
4. Record MCP taxonomy and configuration/lifecycle scope boundaries.
5. Record TOML configuration boundaries and verification expectations including property/fuzz coverage.
6. Link the design to the implementation plan and task registry.

**Green Phase (required)**

Command: `scripts/doc-lint.sh --path docs/plans/2026-03-11-chat-first-tui-design.md --strict-new && scripts/check-tasks-registry.sh`
Expected: design doc passes strict lint and task registry validation.

**Refactor Phase (optional but controlled)**

Allowed scope: wording only in `docs/plans/2026-03-11-chat-first-tui-design.md`
Re-run: `scripts/doc-lint.sh --path docs/plans/2026-03-11-chat-first-tui-design.md --strict-new`

**Completion Evidence**

- [x] Preconditions satisfied
- [x] Invariants preserved
- [x] Postconditions met
- [x] Unit, invariant, integration, and property-based checks defined
- [x] CHANGELOG.md updated
