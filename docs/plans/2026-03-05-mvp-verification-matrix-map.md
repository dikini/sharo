# MVP Verification Matrix Map Implementation Plan

Goal: bind MVP verification matrix rows to executable checks and make mapping machine-checkable.
Architecture: maintain a single markdown mapping file with deterministic row keys and test bindings; enforce shape and reference integrity through Bats checks.
Tech Stack: markdown, bats-core, ripgrep, shell checks.
Template-Profile: tdd-strict-v1

---

### Task 1: Map Matrix Rows To Executable Evidence

**Files:**

- Create: `docs/plans/2026-03-05-mvp-verification-matrix-map.md` (this document)
- Create: `scripts/tests/test-mvp-matrix-map.bats`
- Test: `scripts/tests/test-mvp-matrix-map.bats`

**Preconditions**

- MVP matrix exists in `docs/specs/mvp.md`.
- Slice tests for scenarios A/B/C and protocol/CLI surface are present.

**Invariants**

- Each mapping row has a unique `row_key`.
- `test_bindings` is either a test reference or an explicit `not-implemented:<reason>` marker.
- Bound test file paths must exist.

**Postconditions**

- Matrix map is complete for all current matrix rows.
- Bats checks validate uniqueness, non-empty bindings, and existing test references.

**Tests (must exist before implementation)**

Unit:
- `matrix_map_has_unique_row_keys`

Property:
- `matrix_rows_have_test_binding`

Integration:
- `matrix_map_references_existing_tests`

**Red Phase (required before code changes)**

Command: `scripts/run-shell-tests.sh --all`
Expected: matrix map tests fail before map/check implementation.

**Implementation Steps**

1. Add explicit row-key to test-binding map for all matrix rows.
2. Add Bats assertions for uniqueness, required bindings, and file reference validity.
3. Allow explicit non-implemented rows only when reason text is present.

**Green Phase (required)**

Command: `scripts/run-shell-tests.sh --all`
Expected: all matrix-map checks pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `docs/plans/2026-03-05-mvp-verification-matrix-map.md`, `scripts/tests/test-mvp-matrix-map.bats`
Re-run: `scripts/run-shell-tests.sh --all`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated

## Matrix Row Bindings

| row_key | test_bindings | notes |
|---|---|---|
| task-durable-single-state | crates/sharo-core/tests/runtime_types_tests.rs#task_state_supports_scenario_a_transitions | state transition sanity |
| step-ends-explicitly | crates/sharo-daemon/tests/recovery_invariants.rs#step_terminal_state_is_explicit | explicit terminal states |
| policy-before-restricted-exec | crates/sharo-core/tests/policy_tests.rs#restricted_action_requires_policy_decision | policy precheck path |
| denied-step-no-exec | crates/sharo-daemon/tests/approval_flow.rs#approval_resolution_idempotent_by_approval_id | denied path blocks completion |
| route-decision-recorded | crates/sharo-daemon/tests/scenario_a.rs#scenario_a_read_task_succeeds_with_verification_artifact | route artifact and trace |
| trace-before-derived-memory | crates/sharo-daemon/tests/scenario_a.rs#scenario_a_read_task_succeeds_with_verification_artifact | trace present with artifacts |
| policy-approval-durable-records | crates/sharo-daemon/tests/approval_flow.rs#approval_required_step_waits_then_executes_after_approve | approval persisted and resolvable |
| artifact-provenance-queryable | crates/sharo-daemon/tests/scenario_a.rs#scenario_a_read_task_succeeds_with_verification_artifact | artifact linkage |
| coordination-persist-overlap | crates/sharo-daemon/tests/coordination_store.rs#coordination_records_survive_restart | overlap records survive restart |
| overlap-visible-without-dedicated-commands | crates/sharo-daemon/tests/scenario_c_overlap.rs#scenario_c_overlap_is_visible_without_arbitration | task/trace reads expose overlap |
| recovery-visible-uncertainty-overlap | crates/sharo-daemon/tests/recovery_invariants.rs#recovery_preserves_pending_approval_and_conflict_visibility | conflict visibility after restart |
| approval-restart-safe | crates/sharo-daemon/tests/recovery_invariants.rs#recovery_preserves_pending_approval_and_conflict_visibility | pending approvals survive restart |
| restart-restores-durable-task-state | crates/sharo-daemon/tests/recovery_invariants.rs#trace_continuity_preserved_on_restart | same durable task/trace after restart |
| cli-inspects-blocking-reason | crates/sharo-cli/tests/approval_cli.rs#scenario_b_cli_blocked_and_approval_resolution | blocking reason printed |
| capability-manifest-required | not-implemented:mvp-runtime-uses-local-mock-without-manifest-loader | tracked gap for post-MVP hardening |
| binding-remains-opaque | not-implemented:mvp-does-not-expose-binding-handles-yet | tracked gap for post-MVP hardening |
| verification-observability-artifacts-first-class | crates/sharo-daemon/tests/scenario_a.rs#scenario_a_read_task_succeeds_with_verification_artifact | verification/final artifacts persisted |
