# TUI Interaction Loop Implementation Plan

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Invariant, and Integration checks.
> - Use Property-based tests only when a generative framework is actually used.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

Goal: implement a proper interactive event loop for `sharo-tui` with live multiline composer input, `ratatui` full-screen rendering, keybindings, periodic refresh, and event-stream-ready internal structure.
Architecture: keep the daemon and kernel concurrency unchanged while moving `sharo-tui` from snapshot rendering to a single-threaded raw-mode event loop. All UI state stays owned by the main loop; terminal input, refresh ticks, and background daemon results are funneled through typed internal events before state mutation.
Tech Stack: Rust 2024, `crossterm`, `ratatui`, existing `sharo-tui` app/state modules, standard library channels/threads or Tokio-compatible bounded handoff where needed, existing daemon IPC.
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-TUI-INTERACTION-DESIGN-001, TASK-TUI-INTERACTION-PLAN-001

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. This plan's execution and verification contracts.
4. Explicit plan updates recorded in this file.

## Execution Mode

- Mode: execute-with-checkpoints
- Default: execute-with-checkpoints unless user explicitly requests plan-only output.

## Output Contract

- Keep the daemon execution model unchanged.
- Keep UI state mutation on the main event loop only.
- Add tests before implementation for multiline loop state, `ratatui` layout rendering, key handling, and refresh behavior.
- Preserve slash-command and approval behavior while moving to the interactive loop.

## Task Update Contract

- New interaction requirements must map into the event loop, input model, or refresh model before implementation continues.
- No task may introduce direct background-thread mutation of `App` state.

## Completion Gate

- Completion requires an interactive `sharo-tui` binary, green targeted tests, fast-feedback evidence, docs/task sync, and changelog update.

## Model Compatibility Notes

- The interaction-loop implementation must not weaken daemon or kernel concurrency.
- Polling is an internal event source for now, not a claim that daemon-side streaming already exists.

---

### Task 1: Add interaction-loop docs and task tracking

**Files:**

- Create: `docs/plans/2026-03-12-tui-interaction-loop-design.md`
- Create: `docs/plans/2026-03-12-tui-interaction-loop-plan.md`
- Modify: `docs/tasks/tasks.csv`
- Modify: `CHANGELOG.md`

**Preconditions**

- The merged chat-first TUI slice is on `main`.
- The next TUI slice is scoped to interaction-loop behavior.

**Invariants**

- Docs must state that daemon/kernel concurrency is unchanged.
- Design and plan references must be stable and task-backed.

**Postconditions**

- The interaction-loop slice is documented and task-tracked before code changes.

**Tests (must exist before implementation)**

Unit:
- `docs_reference_tui_interaction_loop_scope`

Invariant:
- `docs_preserve_daemon_concurrency_boundary`

Integration:
- `tasks_registry_references_interaction_loop_docs`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `scripts/check-tasks-sync.sh --changed`
Expected: fails until task-registry rows are added for the new docs.

**Implementation Steps**

1. Add design and plan docs for the interaction-loop slice.
2. Add task-registry rows.
3. Record the slice in `CHANGELOG.md`.

**Green Phase (required)**

Command: `scripts/doc-lint.sh --changed --strict-new && scripts/check-tasks-registry.sh && scripts/check-tasks-sync.sh --changed`
Expected: all doc/task checks pass.

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Docs/task checks passing
- CHANGELOG.md updated

### Task 2: Add event-loop, multiline composer, and navigation tests

**Files:**

- Modify: `crates/sharo-tui/src/app.rs`
- Modify: `crates/sharo-tui/src/state.rs`
- Create: `crates/sharo-tui/src/tui_loop.rs`
- Test: `crates/sharo-tui/tests/interaction_loop.rs`
- Test: `crates/sharo-tui/tests/keybindings.rs`
- Test: `crates/sharo-tui/tests/layout_render.rs`

**Preconditions**

- Current `sharo-tui` app/state surfaces are passing.

**Invariants**

- UI state mutations remain single-threaded.
- Background results are applied through explicit events.
- Failed refreshes do not partially mutate visible state.
- Composer-first focus remains the only input target in this slice.

**Postconditions**

- Failing tests define the desired interactive-loop behavior before implementation starts.

**Tests (must exist before implementation)**

Unit:
- `composer_editing_handles_multiline_insert_cursor_and_delete_boundaries`
- `composer_submit_requires_ctrl_enter_and_plain_enter_inserts_newline`
- `screen_keybindings_switch_focus_without_losing_active_session`
- `layout_cursor_position_tracks_multiline_composer_state`

Invariant:
- `background_result_events_are_applied_only_through_main_loop_reducer`

Integration:
- `interactive_loop_submits_chat_and_refreshes_view`
- `interactive_loop_cycles_sessions_without_cross_contamination`
- `interactive_loop_refresh_tick_preserves_consistent_state_on_fetch_failure`
- `ratatui_layout_renders_active_screen_and_composer_regions`

Property-based (optional):
- `multiline_event_reducer_never_leaves_cursor_outside_buffer`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-tui --test interaction_loop -- --nocapture && cargo test -p sharo-tui --test keybindings -- --nocapture`
Expected: new tests fail before event-loop implementation exists.

**Implementation Steps**

1. Add tests for multiline composer editing, keybindings, reducer invariants, `ratatui` layout, and refresh behavior.
2. Verify those tests fail for the expected missing behavior.

**Green Phase (required)**

Command: `cargo test -p sharo-tui --test interaction_loop -- --nocapture && cargo test -p sharo-tui --test keybindings -- --nocapture`
Expected: targeted tests pass.

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Targeted tests passing

### Task 3: Implement the single-threaded interactive TUI loop

**Files:**

- Modify: `crates/sharo-tui/Cargo.toml`
- Modify: `crates/sharo-tui/src/main.rs`
- Modify: `crates/sharo-tui/src/app.rs`
- Modify: `crates/sharo-tui/src/state.rs`
- Create: `crates/sharo-tui/src/layout.rs`
- Create: `crates/sharo-tui/src/tui_loop.rs`
- Modify: `crates/sharo-tui/src/lib.rs`

**Preconditions**

- Event-loop expectations are encoded in failing tests.

**Invariants**

- Daemon/kernel concurrency remains unchanged.
- Raw-mode terminal setup is cleaned up on exit and error.
- Only the main loop mutates `App` state.

**Postconditions**

- `sharo-tui` runs as a proper full-screen interactive loop.
- Multiline composer input, `Ctrl-Enter` submit behavior, keybindings, and refresh events are wired.
- Interactive rendering uses `ratatui` with explicit header/body/status/composer layout and visible cursor placement.
- Snapshot mode remains available for smoke/testing.

**Tests (must exist before implementation)**

Unit:
- `quit_event_restores_terminal_lifecycle_state`
- `multiline_cursor_navigation_preserves_buffer_integrity`
- `ratatui_layout_separates_body_and_composer_without_overwriting_content`

Invariant:
- `main_loop_is_the_only_owner_of_mutable_app_state`

Integration:
- `interactive_loop_renders_chat_sessions_approvals_trace_and_settings`

Property-based (optional):
- `event_reducer_is_deterministic_for_same_event_sequence`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-tui --test interaction_loop -- --nocapture && cargo test -p sharo-tui --test keybindings -- --nocapture`
Expected: targeted tests remain red until the loop is implemented.

**Implementation Steps**

1. Add the internal event types and main-loop runner.
2. Add `ratatui` frame rendering and terminal lifecycle handling.
3. Add multiline composer state, cursor movement, and `Ctrl-Enter` submit handling.
4. Add explicit cursor placement derived from composer state.
5. Add background request handoff and typed completion events.
6. Keep `--once` snapshot rendering for smoke tests.

**Green Phase (required)**

Command: `cargo test -p sharo-tui -- --nocapture && scripts/check-fast-feedback.sh`
Expected: `sharo-tui` tests and fast-feedback pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/sharo-tui/src/*.rs`, `crates/sharo-tui/tests/*.rs`
Re-run: `cargo test -p sharo-tui -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- `sharo-tui` tests passing
- fast-feedback passing
- CHANGELOG.md updated
