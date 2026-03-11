# Chat-First TUI Implementation Plan

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Invariant, and Integration checks.
> - Use Property-based tests only when a generative framework is actually used.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: add a new daemon-backed `sharo-tui` surface for meaningful chat sessions with session switching, inline approvals, slash commands, Agent Skills-based skill activation, and MCP config/status visibility.
Architecture: preserve the daemon as canonical runtime authority and build the TUI as a peer client surface beside `sharo-cli`. Keep conversation rendering as a derived session view over canonical task, artifact, trace, and approval state while adding only the minimum protocol/control-plane surfaces required for interactive UX. Separate skills, MCP servers, runtime approvals, and future capabilities into explicit subsystems to avoid taxonomy collapse.
Tech Stack: Rust 2024, `ratatui` or equivalent terminal UI crate, `crossterm`, serde/serde_json, existing Unix socket IPC, `proptest`, Rust fuzzing targets where parser or payload surfaces justify them.
Template-Profile: tdd-strict-v1

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. This plan's execution and verification contracts.
4. Explicit plan updates recorded in this file.

## Execution Mode

- Mode: execute-with-checkpoints
- Default: execute-with-checkpoints unless user explicitly requests plan-only output.

## Output Contract

- Keep task definitions concrete with exact file paths and commands.
- Preserve daemon-centric architecture and exact-state invariants.
- Add property-based and fuzzing coverage where inputs are variable or adversarial.
- Record any protocol or config tradeoff explicitly instead of silently broadening scope.

## Task Update Contract

- New accepted requirements must be mapped into the relevant TUI, protocol, skills, or MCP tasks before implementation continues.
- No task may silently merge skills, MCP servers, and capabilities into one undifferentiated feature bucket.
- TOML-configurable items must remain distinct from session/runtime state.

## Completion Gate

- The implementation is complete only when daemon protocol, TUI interactions, docs, tests, and changelog entries all align.
- Completion requires fresh fast-feedback evidence and task-registry synchronization.

## Model Compatibility Notes

- Slash commands are operator-visible control actions, not hidden prompt conventions.
- Skills follow Agent Skills progressive disclosure and bounded recursive discovery.
- Transcript rendering remains derived from canonical state and must be tested as such.

---

### Task 1: Specify the TUI/control-plane slice in canonical docs and task registry

**Files:**

- Modify: `docs/specs/mvp.md`
- Modify: `docs/tasks/tasks.csv`
- Modify: `docs/plans/2026-03-11-chat-first-tui-design.md`
- Modify: `docs/plans/2026-03-11-chat-first-tui-implementation-plan.md`
- Test: `scripts/doc-lint.sh`, `scripts/check-tasks-registry.sh`, `scripts/check-tasks-sync.sh`

**Preconditions**

- `docs/specs/mvp.md` is the active canonical spec for the current runtime posture.
- The design note exists and captures the accepted TUI, skills, and MCP direction.

**Invariants**

- Canonical MVP invariants are preserved.
- The TUI is described as a peer surface, not a canonical runtime replacement.
- Property and fuzzing expectations are explicit where applicable.

**Postconditions**

- Canonical docs reflect the new chat-first TUI direction and control-plane additions.
- Task registry contains plan/design tracking rows for the new work.

**Tests (must exist before implementation)**

Unit:
- `docs_reference_chat_first_tui_surface`

Invariant:
- `docs_preserve_daemon_as_canonical_runtime`

Integration:
- `tasks_registry_references_tui_design_and_plan_sources`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `scripts/check-tasks-sync.sh --changed`
Expected: fails until new task-registry rows are added for the design and implementation plan sources.

**Implementation Steps**

1. Update `docs/specs/mvp.md` with the approved post-MVP TUI/control-plane direction and skills/MCP boundaries.
2. Add task-registry rows for `TASK-TUI-DESIGN-001` and `TASK-TUI-PLAN-001`.
3. Ensure the design and plan docs carry stable task references.

**Green Phase (required)**

Command: `scripts/doc-lint.sh --changed --strict-new && scripts/check-tasks-registry.sh && scripts/check-tasks-sync.sh --changed`
Expected: docs lint and task registry checks pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `docs/specs/mvp.md`, `docs/tasks/tasks.csv`, `docs/plans/2026-03-11-chat-first-tui-design.md`, `docs/plans/2026-03-11-chat-first-tui-implementation-plan.md`
Re-run: `scripts/doc-lint.sh --changed --strict-new && scripts/check-tasks-registry.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 2: Add daemon protocol and derived session-view surfaces for chat-first UX

**Files:**

- Modify: `crates/sharo-core/src/protocol.rs`
- Modify: `crates/sharo-core/src/runtime_types.rs`
- Modify: `crates/sharo-daemon/src/main.rs`
- Modify: `crates/sharo-daemon/src/store.rs`
- Create: `crates/sharo-daemon/src/control_plane.rs`
- Test: `crates/sharo-daemon/tests/daemon_ipc.rs`
- Test: `crates/sharo-cli/tests/scenario_a_cli.rs`

**Preconditions**

- Existing IPC request/response handling and session/task persistence are passing.
- No transcript-native canonical store is introduced.

**Invariants**

- Daemon remains canonical runtime authority.
- Session views are derived from canonical task, artifact, trace, and approval state.
- Session-oriented retrieval stays bounded to recent-task windows so control-plane payloads are transport-safe by default.
- Accepted session ids become durable session records before session-oriented retrieval depends on them.
- Approval and policy semantics remain unchanged.

**Postconditions**

- Protocol supports session listing, session task retrieval, and compact runtime/session view retrieval needed by the TUI.
- Active session approval state is retrievable without shelling into raw object-by-object fetches.
- Implicit or previously unregistered submit flows remain discoverable through session listing and session-view retrieval.
- Session-task and session-view retrieval can be requested with an explicit limit and are clamped by the daemon to a bounded maximum.

**Tests (must exist before implementation)**

Unit:
- `session_view_surfaces_latest_result_preview`

Invariant:
- `derived_session_view_never_mutates_canonical_store_state`

Integration:
- `list_sessions_returns_recent_activity_order`
- `session_view_surfaces_pending_approval_for_active_conversation`

Property-based (optional):
- `session_task_order_is_monotonic_under_valid_store_state` using `proptest`

Fuzz:
- `daemon_request_session_views_reject_or_parse_arbitrary_json_without_panicking` via `crates/sharo-core/fuzz/fuzz_targets/daemon_request_session_views.rs`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-daemon daemon_ipc -- --nocapture`
Expected: targeted IPC/session-view tests fail before new protocol variants and handlers are implemented.

**Implementation Steps**

1. Add protocol variants for session listing and derived session views.
2. Implement a control-plane module to shape session-oriented retrieval payloads.
3. Extend daemon request handling to serve the new views.
4. Materialize accepted submit session ids into durable session records before control-plane retrieval depends on them.
5. Add bounded task-window shaping, session ordering, and active-approval visibility coverage.
6. Add parser fuzz coverage for the new session-oriented daemon request payloads.

**Green Phase (required)**

Command: `cargo test -p sharo-daemon daemon_ipc -- --nocapture && cargo test -p sharo-core ipc_protocol_tests -- --nocapture && cargo test -p sharo-cli scenario_a_cli -- --nocapture`
Expected: targeted daemon, protocol, and CLI integration tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/sharo-core/src/protocol.rs`, `crates/sharo-daemon/src/main.rs`, `crates/sharo-daemon/src/store.rs`, `crates/sharo-daemon/src/control_plane.rs`
Re-run: `cargo test -p sharo-daemon daemon_ipc -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, integration, and property-based checks passing
- CHANGELOG.md updated

### Task 3: Implement Agent Skills discovery, catalog retrieval, and session activation

**Files:**

- Create: `crates/sharo-core/src/skills.rs`
- Create: `crates/sharo-daemon/src/skills.rs`
- Modify: `crates/sharo-daemon/src/config.rs`
- Modify: `crates/sharo-daemon/src/main.rs`
- Modify: `crates/sharo-daemon/src/store.rs`
- Test: `crates/sharo-daemon/tests/daemon_smoke.rs`
- Test: `crates/sharo-core/tests/runtime_types_tests.rs`
- Test: `crates/sharo-daemon/fuzz/` if fuzz target layout is introduced

**Preconditions**

- Protocol can carry catalog/state responses for the TUI.
- Skills are treated as instruction assets, not capabilities.

**Invariants**

- Bounded recursive discovery is deterministic.
- Project scope overrides user scope for identical relative skill ids.
- Full skill contents are not loaded into every session by default.

**Postconditions**

- Daemon discovers skills from configured roots.
- Skill ids are stable relative paths from the root.
- Session-scoped skill activation and deactivation is persisted and inspectable.

**Tests (must exist before implementation)**

Unit:
- `recursive_skill_discovery_finds_bundled_skill_layouts`
- `project_skill_precedence_overrides_user_scope`

Invariant:
- `session_active_skills_do_not_leak_across_sessions`

Integration:
- `list_skills_returns_catalog_without_full_skill_payloads`
- `set_session_skills_persists_activation_state`

Property-based (optional):
- `skill_id_derivation_is_stable_for_valid_relative_paths` using `proptest`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-daemon daemon_smoke -- --nocapture`
Expected: skills-specific daemon tests fail before discovery and activation surfaces exist.

**Implementation Steps**

1. Add TOML-configurable skills roots, max depth, and trust-policy knobs.
2. Implement bounded recursive discovery and catalog shaping.
3. Add session-scoped active skill storage and protocol handlers.
4. Add property-based coverage for relative-path identity derivation.
5. Add a fuzz target for skill discovery path and metadata parsing if a fuzz harness is introduced.

**Green Phase (required)**

Command: `cargo test -p sharo-daemon daemon_smoke -- --nocapture && cargo test -p sharo-core runtime_types_tests -- --nocapture`
Expected: targeted daemon and core tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/sharo-core/src/skills.rs`, `crates/sharo-daemon/src/skills.rs`, `crates/sharo-daemon/src/config.rs`, `crates/sharo-daemon/src/store.rs`
Re-run: `cargo test -p sharo-daemon daemon_smoke -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, integration, and property-based checks passing
- CHANGELOG.md updated

### Task 4: Implement MCP registry config and runtime status surfaces

**Files:**

- Create: `crates/sharo-core/src/mcp.rs`
- Create: `crates/sharo-daemon/src/mcp_registry.rs`
- Modify: `crates/sharo-daemon/src/config.rs`
- Modify: `crates/sharo-daemon/src/main.rs`
- Test: `crates/sharo-daemon/tests/daemon_smoke.rs`

**Preconditions**

- Daemon config loading already supports model and connector settings.
- MCP support remains configuration/status-focused in this slice.

**Invariants**

- MCP servers remain distinct from skills and capabilities.
- Enable/disable state is explicit and inspectable.
- Invalid MCP config fails clearly before request-time ambiguity.

**Postconditions**

- MCP server definitions can be listed through protocol surfaces.
- Operators can enable or disable configured servers.
- Runtime status exposes health/diagnostic summaries for the TUI settings screen.

**Tests (must exist before implementation)**

Unit:
- `mcp_registry_shapes_server_status_summary`

Invariant:
- `disabled_mcp_server_never_reports_running_status`

Integration:
- `list_mcp_servers_returns_configured_statuses`
- `update_mcp_server_state_is_persisted_and_retrievable`

Property-based (optional):
- `mcp_server_ids_are_unique_after_valid_config_parse` using `proptest`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-daemon daemon_smoke -- --nocapture`
Expected: MCP-specific daemon tests fail before registry and status surfaces exist.

**Implementation Steps**

1. Add MCP server config types and validation rules.
2. Implement MCP registry/status shaping and state transitions.
3. Add protocol handlers for listing and toggling servers.
4. Add property-based coverage for unique config identity and state invariants.

**Green Phase (required)**

Command: `cargo test -p sharo-daemon daemon_smoke -- --nocapture`
Expected: MCP registry and status tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/sharo-core/src/mcp.rs`, `crates/sharo-daemon/src/mcp_registry.rs`, `crates/sharo-daemon/src/config.rs`
Re-run: `cargo test -p sharo-daemon daemon_smoke -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, integration, and property-based checks passing
- CHANGELOG.md updated

### Task 5: Create the `sharo-tui` crate and base application shell

**Files:**

- Create: `crates/sharo-tui/Cargo.toml`
- Create: `crates/sharo-tui/src/main.rs`
- Create: `crates/sharo-tui/src/app.rs`
- Create: `crates/sharo-tui/src/state.rs`
- Modify: `Cargo.toml`
- Test: `crates/sharo-tui/tests/tui_smoke.rs`

**Preconditions**

- Protocol/control-plane surfaces needed by the TUI exist.
- Workspace remains Rust 2024 and `rust-version >= 1.93` compliant.

**Invariants**

- TUI remains a client of daemon IPC.
- No runtime state is duplicated as canonical truth in the TUI.
- Screen switching is explicit and stable.

**Postconditions**

- Workspace includes a compilable `sharo-tui` crate.
- TUI starts, connects to the daemon, and exposes chat-first shell navigation.

**Tests (must exist before implementation)**

Unit:
- `default_screen_is_chat`

Invariant:
- `active_session_state_is_distinct_from_screen_focus_state`

Integration:
- `tui_starts_and_renders_chat_shell`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-tui -- --nocapture`
Expected: fails before the crate and smoke test scaffolding exist.

**Implementation Steps**

1. Add `sharo-tui` workspace crate and dependencies.
2. Implement base app shell, screen enum, and daemon IPC client wrapper.
3. Add smoke coverage for startup and screen state.

**Green Phase (required)**

Command: `cargo test -p sharo-tui -- --nocapture`
Expected: TUI smoke tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/sharo-tui/`
Re-run: `cargo test -p sharo-tui -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 6: Implement chat-first interaction, session switching, and inline approvals

**Files:**

- Create: `crates/sharo-tui/src/screens/chat.rs`
- Create: `crates/sharo-tui/src/screens/sessions.rs`
- Create: `crates/sharo-tui/src/screens/approvals.rs`
- Modify: `crates/sharo-tui/src/app.rs`
- Modify: `crates/sharo-tui/src/state.rs`
- Test: `crates/sharo-tui/tests/chat_flow.rs`

**Preconditions**

- TUI shell exists.
- Daemon session views and approval retrieval work.

**Invariants**

- Chat transcript is rendered from derived daemon-backed state.
- Active-session switching never merges or rewrites transcript state.
- Approval gates in the active conversation are surfaced immediately.

**Postconditions**

- Operators can open/create sessions, switch between sessions, submit turns, and see active approval blocks inline in chat.
- Sessions screen and approvals screen reflect the same canonical daemon state as chat.

**Tests (must exist before implementation)**

Unit:
- `chat_view_renders_inline_approval_block_for_active_turn`

Invariant:
- `session_switch_keeps_transcripts_disjoint`

Integration:
- `submit_turn_updates_active_session_chat_view`
- `switching_sessions_changes_active_chat_transcript`
- `approval_resolution_refreshes_current_chat_view`

Property-based (optional):
- `session_switch_sequence_never_cross_contaminates_active_transcript` using `proptest`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-tui chat_flow -- --nocapture`
Expected: chat/session/approval flow tests fail before implementation.

**Implementation Steps**

1. Implement chat rendering from derived session views.
2. Add session picker and fast active-session switching.
3. Render inline approval callouts and approval resolution refresh paths.
4. Add property-based session-switch sequence coverage.

**Green Phase (required)**

Command: `cargo test -p sharo-tui chat_flow -- --nocapture`
Expected: chat/session/approval tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/sharo-tui/src/screens/chat.rs`, `crates/sharo-tui/src/screens/sessions.rs`, `crates/sharo-tui/src/screens/approvals.rs`, `crates/sharo-tui/src/app.rs`
Re-run: `cargo test -p sharo-tui chat_flow -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, integration, and property-based checks passing
- CHANGELOG.md updated

### Task 7: Implement slash-command parsing and command dispatch

**Files:**

- Create: `crates/sharo-tui/src/commands.rs`
- Modify: `crates/sharo-tui/src/screens/chat.rs`
- Modify: `crates/sharo-tui/src/app.rs`
- Test: `crates/sharo-tui/tests/slash_commands.rs`
- Test: `crates/sharo-tui/fuzz/` if fuzz target layout is introduced

**Preconditions**

- TUI chat composer exists.
- Daemon protocol operations for session, approval, skill, MCP, and model queries exist or are stubbed for integration.

**Invariants**

- Slash commands do not silently degrade into normal chat turns.
- Approval and policy semantics are never bypassed.
- Invalid command input produces stable operator-visible errors.

**Postconditions**

- Chat composer supports slash-command parsing and execution for session, approval, skill, MCP, and model command families.
- Known operator actions can be performed without screen navigation.

**Tests (must exist before implementation)**

Unit:
- `parse_slash_command_with_argument_vector`
- `invalid_slash_command_returns_structured_error`

Invariant:
- `slash_command_dispatch_never_uses_chat_submit_path_for_control_actions`

Integration:
- `approve_command_resolves_pending_approval`
- `session_switch_command_changes_active_session`
- `skill_enable_command_updates_session_skill_state`

Property-based (optional):
- `slash_parser_round_trips_valid_argument_boundaries` using `proptest`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-tui slash_commands -- --nocapture`
Expected: slash-command tests fail before parser and dispatch implementation.

**Implementation Steps**

1. Implement slash-command lexer/parser and typed command enum.
2. Route commands to explicit daemon operations.
3. Add integration coverage for approval, session switching, and skill activation.
4. Add a fuzz target for slash-command parsing edge cases and malformed inputs.

**Green Phase (required)**

Command: `cargo test -p sharo-tui slash_commands -- --nocapture`
Expected: slash-command tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/sharo-tui/src/commands.rs`, `crates/sharo-tui/src/screens/chat.rs`, `crates/sharo-tui/src/app.rs`
Re-run: `cargo test -p sharo-tui slash_commands -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, integration, and property-based checks passing
- CHANGELOG.md updated

### Task 8: Implement settings and exact-inspection screens for skills, MCP, and artifacts

**Files:**

- Create: `crates/sharo-tui/src/screens/artifacts.rs`
- Create: `crates/sharo-tui/src/screens/settings.rs`
- Modify: `crates/sharo-tui/src/app.rs`
- Test: `crates/sharo-tui/tests/settings_and_artifacts.rs`

**Preconditions**

- Skills and MCP protocol surfaces exist.
- Artifact and trace retrieval are already available.

**Invariants**

- Settings screen reflects config/runtime state but does not become config authority.
- Artifact/trace inspection remains exact-state-first.
- Skills, MCP, model profile, and approvals remain distinct UI concepts.

**Postconditions**

- Settings screen displays active model, skill catalog and activation state, MCP server status, and runtime warnings.
- Artifact/trace screen exposes exact runtime records behind the current or selected turn.

**Tests (must exist before implementation)**

Unit:
- `settings_screen_groups_skills_mcp_and_model_separately`

Invariant:
- `artifact_screen_uses_exact_record_ids_from_daemon_state`

Integration:
- `settings_screen_renders_skill_and_mcp_status`
- `artifact_screen_renders_route_and_final_result_records`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-tui settings_and_artifacts -- --nocapture`
Expected: settings and artifact tests fail before implementation.

**Implementation Steps**

1. Implement settings screen groups and data wiring.
2. Implement artifact/trace screen with exact record references.
3. Add integration coverage for screen rendering against daemon-backed fixtures.

**Green Phase (required)**

Command: `cargo test -p sharo-tui settings_and_artifacts -- --nocapture`
Expected: settings and artifact tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/sharo-tui/src/screens/artifacts.rs`, `crates/sharo-tui/src/screens/settings.rs`, `crates/sharo-tui/src/app.rs`
Re-run: `cargo test -p sharo-tui settings_and_artifacts -- --nocapture`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 9: Run full verification, add changelog evidence, and preserve fast-feedback freshness

**Files:**

- Modify: `CHANGELOG.md`
- Modify: `docs/tasks/tasks.csv`
- Test: `scripts/check-fast-feedback.sh`
- Test: `cargo test --workspace`

**Preconditions**

- All prior TUI, protocol, skills, and MCP tasks are implemented.

**Invariants**

- Docs, task registry, changelog, and code remain synchronized.
- Fast-feedback marker freshness is preserved.

**Postconditions**

- Changelog records the new TUI/control-plane slice.
- Full verification evidence exists for workspace tests and policy gates.

**Tests (must exist before implementation)**

Unit:
- `tasks_registry_entries_have_existing_sources`

Invariant:
- `fast_feedback_marker_matches_current_tree_state`

Integration:
- `scripts/check-fast-feedback.sh` passes end-to-end
- `cargo test --workspace` passes end-to-end

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `scripts/check-fast-feedback.sh`
Expected: fails until implementation, docs, and changelog/task-registry updates are complete.

**Implementation Steps**

1. Update `CHANGELOG.md` with the TUI/control-plane additions.
2. Mark task registry status accurately for completed work.
3. Run full fast-feedback and workspace tests.
4. Preserve verification outputs as completion evidence.

**Green Phase (required)**

Command: `scripts/check-fast-feedback.sh && cargo test --workspace`
Expected: all policy checks and workspace tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `CHANGELOG.md`, `docs/tasks/tasks.csv`, final polish for already-implemented code paths only
Re-run: `scripts/check-fast-feedback.sh && cargo test --workspace`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated
