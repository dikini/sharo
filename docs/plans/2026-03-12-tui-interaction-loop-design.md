# TUI Interaction Loop Design

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Invariant, and Integration checks.
> - Use Property-based tests only when a generative framework is actually used.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

Updated: 2026-03-12
Status: active
Owner: runtime
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-TUI-INTERACTION-DESIGN-001, TASK-TUI-INTERACTION-PLAN-001

## Goal

Turn the merged `sharo-tui` snapshot shell into a proper interactive terminal application with live input, screen/key navigation, periodic refresh, and an event-stream-ready internal architecture.

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
- Keep interaction-loop responsibilities on the TUI side only.
- Preserve slash-command, approval, and derived-transcript behavior while improving interaction.

## Architecture

Keep the daemon and kernel execution model unchanged. The interaction loop is a TUI-only concern. `sharo-tui` should move to a single-threaded UI event loop that owns all visible state and consumes terminal events, tick events, and background daemon result events. Background daemon calls remain concurrent and bounded, but they must only communicate back to the TUI through typed events so the UI state stays deterministic and race-free.

## Accepted Requirements

1. The TUI loop is single-threaded; the daemon is not.
2. The loop runs in raw mode with alternate-screen rendering.
3. The TUI supports live multiline input editing and slash-command submission from the composer.
4. The TUI supports better navigation with explicit keybindings for screen switching, session switching, refresh, and quit.
5. Refresh is event-stream-ready:
   - immediate after local actions
   - periodic polling until daemon streaming exists
6. Only the main UI loop mutates presentation state.
7. The composer remains the primary input target; pane focus is deferred.
8. Interactive rendering should use `ratatui` frame layout instead of plain text redraws.

## Boundaries

- In scope:
- terminal event loop
- render loop
- multiline input composer/editing
- keybinding dispatch
- periodic refresh
- bounded background daemon request execution
- `ratatui`-based frame composition for header, body, status, and composer areas
- Out of scope:
  - daemon streaming protocol
  - transcript-canonical storage changes
  - kernel or connector concurrency changes
  - MCP execution-surface expansion

## Event Model

The TUI should use an internal event enum with at least:

- terminal key/input events
- periodic tick events
- background command/result completion events
- resize events
- quit events

Terminal input and background results must be normalized into this internal event stream before state mutation.

## State Rules

1. `App` remains the owner of visible TUI state.
2. Background work may compute results, but it must not mutate `App` directly.
3. Failed refreshes must not leave partial visible-state updates.
4. Screen focus, composer state, active session, and cached inspection state must remain internally consistent after every event application.

## UX Contract

- default screen remains `Chat`
- slash commands remain first-class
- approvals stay visible inline in `Chat`
- operators can switch screens without losing active composer/session context
- session navigation must be faster than going through a browse-only screen
- `Enter` inserts a newline in the composer
- `Ctrl-Enter` submits the current composer buffer
- global navigation keys remain active without introducing pane-focus state
- the active insertion point must be visually explicit in the interactive UI
- chat, sessions, approvals, trace/artifacts, and settings remain separate switchable views inside one frame layout

## Testing Focus

- reducer/event transition determinism
- keybinding correctness
- multiline composer editing behavior
- multiline cursor movement across line boundaries
- `Ctrl-Enter` submit semantics
- `ratatui` layout correctness for active screen/body/composer separation
- visible cursor placement derived from composer state
- refresh/tick behavior
- no partial-state mutation on failed background refresh
- terminal lifecycle cleanup on exit

### Task 1: Define the interaction-loop architecture contract

**Files:**

- Create: `docs/plans/2026-03-12-tui-interaction-loop-design.md`
- Create: `docs/plans/2026-03-12-tui-interaction-loop-plan.md`
- Modify: `docs/tasks/tasks.csv`

**Preconditions**

- The chat-first TUI shell is merged and documented.
- The next slice is limited to interaction-loop behavior.

**Invariants**

- The daemon and kernel remain concurrent and unchanged in execution model.
- The TUI main loop is the sole mutable owner of presentation state.
- Background results must re-enter through typed events.

**Postconditions**

- The accepted interaction-loop architecture is documented in canonical repo docs.

**Tests (must exist before implementation)**

Unit:
- `docs_define_single_threaded_tui_loop_boundary`

Invariant:
- `docs_preserve_daemon_concurrency_boundary`

Integration:
- `tasks_registry_references_interaction_loop_design_and_plan`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `scripts/check-tasks-sync.sh --changed`
Expected: fails until task registry entries and doc references are added.

**Implementation Steps**

1. Document the event-loop architecture, state ownership, and UX contract.
2. Document the explicit daemon concurrency boundary.
3. Reference the design and plan from the task registry.

**Green Phase (required)**

Command: `scripts/doc-lint.sh --changed --strict-new && scripts/check-tasks-registry.sh && scripts/check-tasks-sync.sh --changed`
Expected: docs and task checks pass.

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Docs/task checks passing

## Task Update Contract

- New interaction requirements must be mapped into the event loop, input model, or refresh model before implementation continues.
- No follow-on plan may silently weaken the daemon concurrency boundary.

## Completion Gate

- This design is complete only when the accepted interaction-loop boundary, state rules, UX contract, and testing focus are documented and task-backed.

## Model Compatibility Notes

- The phrase "single-threaded event loop" applies to the TUI loop only.
- The daemon/kernel concurrency model must remain explicitly out of scope for this slice.
