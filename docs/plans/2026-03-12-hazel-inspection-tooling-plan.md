# Hazel Inspection Tooling Implementation Plan

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Invariant, and Integration checks.
> - Use Property-based tests only when a generative framework is actually used.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: implement daemon/TUI-first Hazel inspection and safe control tooling with typed daemon APIs, durable operator-visible records, and TUI/CLI client surfaces.
Architecture: keep Hazel canonical logic in existing Hazel crates while adding daemon control-plane handlers and response types for status, inspection, preview, validation, submission, and sleep-job operations. Keep all client surfaces as thin daemon consumers and preserve proposal-producing-only sleep semantics.
Tech Stack: Rust 2024, existing `sharo-core` protocol, `sharo-daemon`, `sharo-tui`, `sharo-cli`, `serde`, current Hazel crates.
Template-Profile: tdd-strict-v1
Updated: 2026-03-12
Status: completed

Task-Registry-Refs: TASK-HAZEL-INSPECTION-PLAN-001, TASK-HAZEL-INSPECTION-SPEC-001, TASK-HAZEL-INSPECTION-DESIGN-001

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
- Preserve daemon authority and Hazel deterministic validation semantics.
- Add tests before implementation for protocol, daemon, and client surfaces.
- Record unresolved risks instead of silently broadening scope.

## Task Update Contract

- New operator requirements must map into protocol, daemon records, or client surfaces before implementation continues.
- No task may introduce direct client-side Hazel storage access.

## Completion Gate

- Completion requires spec/design/task alignment, green targeted tests, fresh fast-feedback evidence, and changelog/task-registry updates.

## Model Compatibility Notes

- Hazel preview actions are control-plane operations, not prompt-injection runtime hooks.
- TUI Hazel views remain daemon-derived screens, not canonical stores.

### Task 1: Add Hazel inspection/control docs and task tracking

**Files:**

- Create: `docs/specs/hazel-inspection-tooling.md`
- Create: `docs/plans/2026-03-12-hazel-inspection-tooling-design.md`
- Create: `docs/plans/2026-03-12-hazel-inspection-tooling-plan.md`
- Modify: `docs/tasks/tasks.csv`
- Modify: `CHANGELOG.md`

**Preconditions**

- Hazel structured-memory subsystem spec remains active.
- The approved direction is daemon/TUI-first with safe control actions included.

**Invariants**

- Docs preserve daemon authority and sleep proposal-only constraints.
- Task references remain stable and deterministic.

**Postconditions**

- Hazel inspection/control work is documented and task-backed before code changes.

**Tests (must exist before implementation)**

Unit:
- `docs_reference_hazel_inspection_control_scope`

Invariant:
- `docs_preserve_daemon_owned_hazel_authority`

Integration:
- `tasks_registry_references_hazel_inspection_spec_design_and_plan`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `scripts/check-tasks-sync.sh --changed`
Expected: fails until new task-registry rows are added.

**Implementation Steps**

1. Add the Hazel inspection/control spec.
2. Add the design note and implementation plan.
3. Add task-registry rows and changelog entry.

**Green Phase (required)**

Command: `scripts/doc-lint.sh --changed --strict-new && scripts/check-tasks-registry.sh && scripts/check-tasks-sync.sh --changed`
Expected: doc/task checks pass.

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Docs/task checks passing
- CHANGELOG.md updated

### Task 2: Add daemon protocol surfaces for Hazel status and inspection

**Files:**

- Modify: `crates/sharo-core/src/protocol.rs`
- Modify: `crates/sharo-core/tests/ipc_protocol_tests.rs`
- Modify: `crates/sharo-daemon/src/main.rs`
- Create: `crates/sharo-daemon/src/hazel_control_plane.rs`
- Modify: `crates/sharo-daemon/tests/daemon_ipc.rs`

**Preconditions**

- Existing daemon control-plane request/response handling is passing.

**Invariants**

- Hazel inspection remains daemon-owned and bounded.
- Hazel control-plane views remain distinct from task/trace/artifact retrieval.

**Postconditions**

- The protocol supports Hazel status, list/get cards, list/get proposal batches, and list/get sleep jobs.
- Daemon handlers shape transport-safe Hazel inspection views.

**Tests (must exist before implementation)**

Unit:
- `hazel_status_response_is_bounded`
- `hazel_card_view_preserves_provenance_fields`

Invariant:
- `hazel_inspection_responses_never_expose_unbounded_payloads`

Integration:
- `hazel_list_cards_returns_transport_safe_view`
- `hazel_get_proposal_batch_returns_exact_provenance_summary`
- `hazel_list_sleep_jobs_returns_bounded_statuses`

Property-based (optional):
- `hazel_card_listing_order_is_deterministic_for_same_store_state`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-daemon daemon_ipc hazel_ -- --nocapture`
Expected: targeted Hazel daemon tests fail before new protocol variants and handlers exist.

**Implementation Steps**

1. Add Hazel inspection request/response types in `sharo-core`.
2. Add daemon Hazel inspection response shaping.
3. Wire daemon request handling for Hazel status and list/get surfaces.
4. Add protocol and daemon tests for bounded views and deterministic ordering.

**Green Phase (required)**

Command: `cargo test -p sharo-core ipc_protocol_tests -- --nocapture && cargo test -p sharo-daemon daemon_ipc hazel_ -- --nocapture`
Expected: targeted protocol and daemon tests pass.

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Targeted tests passing

### Task 3: Add daemon safe control actions and durable Hazel action records

**Files:**

- Modify: `crates/sharo-core/src/protocol.rs`
- Modify: `crates/sharo-hazel-core/src/ingest.rs`
- Modify: `crates/sharo-hazel-core/src/sleep.rs`
- Modify: `crates/sharo-daemon/src/main.rs`
- Modify: `crates/sharo-daemon/src/hazel_control_plane.rs`
- Modify: `crates/sharo-daemon/src/store.rs`
- Modify: `crates/sharo-daemon/tests/daemon_ipc.rs`

**Preconditions**

- Hazel inspection request/response surfaces exist.

**Invariants**

- Retrieval preview is non-mutating.
- Validation remains explicit and fail-closed.
- Sleep jobs remain proposal-producing only.

**Postconditions**

- Daemon supports retrieval preview, proposal validation, proposal submission, and sleep-job enqueue/cancel.
- Durable Hazel action records are inspectable after completion/failure.

**Tests (must exist before implementation)**

Unit:
- `hazel_preview_request_rejects_oversized_limits`
- `hazel_validate_batch_rejects_unknown_policy_ids_in_strict_mode`

Invariant:
- `hazel_preview_returns_without_canonical_write_side_effects`
- `hazel_sleep_jobs_remain_proposal_producing_only`

Integration:
- `hazel_preview_returns_derived_payload_without_canonical_write`
- `hazel_submit_batch_persists_submission_outcome_record`
- `hazel_cancel_sleep_job_stops_future_proposal_production`

Property-based (optional):
- `hazel_preview_for_same_inputs_is_deterministic`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-daemon daemon_ipc hazel_preview hazel_submit hazel_sleep -- --nocapture`
Expected: targeted Hazel action tests fail before handlers and durable records exist.

**Implementation Steps**

1. Add safe-action protocol variants.
2. Add daemon request handlers and durable Hazel action records in the store.
3. Wire Hazel core validation/retrieval/sleep helpers into daemon handlers.
4. Add daemon tests for preview, validate, submit, and sleep-job control flows.

**Green Phase (required)**

Command: `cargo test -p sharo-daemon daemon_ipc hazel_ -- --nocapture`
Expected: targeted Hazel action tests pass.

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Targeted tests passing

### Task 4: Add TUI and CLI Hazel operator surfaces

**Files:**

- Modify: `crates/sharo-tui/src/app.rs`
- Modify: `crates/sharo-tui/src/commands.rs`
- Modify: `crates/sharo-tui/src/state.rs`
- Modify: `crates/sharo-tui/src/layout.rs`
- Modify: `crates/sharo-tui/src/tui_loop.rs`
- Modify: `crates/sharo-tui/tests/slash_commands.rs`
- Create: `crates/sharo-tui/tests/hazel_screen.rs`
- Modify: `crates/sharo-cli/src/main.rs`
- Create: `crates/sharo-cli/tests/hazel_cli.rs`

**Preconditions**

- Daemon Hazel inspection/control-plane APIs exist.

**Invariants**

- TUI and CLI remain daemon clients only.
- Hazel views remain separate from chat/settings screens.

**Postconditions**

- `sharo-tui` has a Hazel screen and Hazel slash-command flows.
- `sharo-cli` exposes Hazel inspection/control commands over daemon APIs.

**Tests (must exist before implementation)**

Unit:
- `hazel_tui_screen_renders_daemon_returned_status_and_counts`

Invariant:
- `hazel_slash_commands_dispatch_only_through_daemon_control_plane`

Integration:
- `hazel_slash_preview_command_surfaces_preview_record`
- `hazel_cli_submit_command_surfaces_submission_outcome`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-tui hazel_ -- --nocapture && cargo test -p sharo-cli hazel_ -- --nocapture`
Expected: targeted client tests fail before Hazel operator surfaces exist.

**Implementation Steps**

1. Add Hazel TUI state/rendering and slash-command entries.
2. Add Hazel CLI commands matching daemon control-plane contracts.
3. Add client tests for rendering and command dispatch.

**Green Phase (required)**

Command: `cargo test -p sharo-tui -- --nocapture && cargo test -p sharo-cli -- --nocapture`
Expected: TUI and CLI tests pass.

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Targeted tests passing

### Task 5: Run full verification and close the slice

**Files:**

- Modify: `CHANGELOG.md`
- Modify: `docs/tasks/tasks.csv`

**Preconditions**

- Protocol, daemon, TUI, and CLI Hazel tooling work is implemented.

**Invariants**

- Verification evidence is fresh.
- Task/changelog state matches delivered work.

**Postconditions**

- Hazel inspection tooling is fully verified and documented as complete.

**Tests (must exist before implementation)**

Unit:
- `changelog_mentions_hazel_inspection_tooling`

Invariant:
- `task_registry_marks_hazel_inspection_items_done_only_after_verification`

Integration:
- `full_fast_feedback_passes_with_hazel_inspection_tooling`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `scripts/check-fast-feedback.sh`
Expected: may fail until all implementation and task/changelog updates are complete.

**Implementation Steps**

1. Update changelog and task registry completion notes.
2. Run full fast-feedback on the final tree state.
3. Re-run targeted Hazel protocol/daemon/TUI/CLI tests if any last adjustments were made.

**Green Phase (required)**

Command: `scripts/check-fast-feedback.sh`
Expected: fast-feedback passes on the final tree state.

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- fast-feedback passing
- CHANGELOG.md updated
