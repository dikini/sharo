# Hazel Inspection Tooling

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Define tests before implementation work.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Invariant, and Integration checks.
> - Use Property-based tests only when a generative framework is actually used.
> - Run `scripts/doc-lint.sh` before commit.

Updated: 2026-03-12
Status: active
Owner: runtime
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-HAZEL-INSPECTION-SPEC-001

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. This spec's task contracts and invariants.
4. In-task updates recorded explicitly in this document.

## Output Contract

- Preserve daemon authority for all Hazel inspection and control-plane access.
- Keep Hazel inspection distinct from task execution, MCP configuration, and exact task/trace/artifact retrieval.
- Expose only typed, bounded, operator-safe inspection and control surfaces.

## Evidence / Verification Contract

- Every completion claim must cite verification commands/results in `## Verification`.
- Conflicting evidence must be called out explicitly before task closure.
- If verification cannot run, record why and the residual risk.

## Model Compatibility Notes

- XML-style delimiter blocks are optional structure aids only.
- Critical daemon-authority and safe-control constraints must also be stated in plain language.
- TUI-visible Hazel data remains derived from daemon-returned control-plane records, not locally reconstructed state.

## Purpose

Define daemon/TUI-first Hazel inspection and safe control tooling for Sharo. This slice introduces operator-facing control-plane surfaces for inspecting Hazel memory state and exercising bounded safe actions without bypassing deterministic Hazel validation or mutating canonical memory opaquely.

## Scope

### In Scope

- Daemon protocol surfaces for Hazel status, card inspection, proposal-batch inspection, retrieval preview, validation, submission, and sleep-job inspection/control.
- Durable daemon-visible records for Hazel preview, validation, submission, and sleep-job outcomes.
- TUI and CLI client surfaces that consume daemon Hazel control-plane APIs.
- Bounded safe control actions:
  - retrieval preview
  - proposal-batch validation
  - proposal-batch submission
  - sleep-job enqueue/list/get/cancel

### Out of Scope

- Direct local TUI or CLI access to Hazel storage bypassing the daemon.
- Opaque direct canonical-memory mutation from operator actions.
- Rich free-form memory editing UI.
- Broad document-extraction pipelines beyond explicit proposal-batch submission.
- Sleep jobs that write canonical memory directly without proposal batches.

## Core Terms

- `Hazel Status`: compact daemon-returned summary of Hazel availability, configured capabilities, and current limits.
- `Hazel Card View`: bounded inspectable view of canonical Hazel memory cards and their provenance/state metadata.
- `Hazel Proposal Batch View`: inspectable representation of proposal-batch provenance, shape, validation state, and submission outcome.
- `Hazel Retrieval Preview`: derived recollection preview produced for operator inspection without prompt injection or canonical write side effects.
- `Hazel Sleep Job`: bounded background workflow that produces proposal batches only.
- `Hazel Action Record`: durable exact record for preview, validation, submission, or sleep-job lifecycle events.

## Interfaces / Contracts

- The daemon MUST be the only authority that exposes Hazel inspection and safe control-plane operations to clients.
- Hazel control-plane requests MUST be typed, bounded, and transport-safe like other daemon control-plane surfaces.
- TUI and CLI surfaces MUST consume daemon responses; they MUST NOT read or mutate Hazel state directly.
- Retrieval preview MUST:
  - run Hazel retrieval/validation logic
  - return derived recollection payload plus validation/provenance metadata
  - avoid prompt injection side effects
  - avoid canonical Hazel writes
- Proposal-batch validation MUST produce an operator-visible validation result without implicit submission.
- Proposal-batch submission MUST remain explicit, provenance-backed, and validator-gated.
- Sleep-job enqueue/list/get/cancel MUST preserve the existing Hazel rule that sleep jobs are proposal-producing only.
- Canceling a sleep job MUST stop further proposal production but MUST NOT silently remove already produced durable records.
- Unknown policy ids, invalid proposal shapes, oversized previews, and budget violations MUST fail closed and remain operator-visible.
- Hazel inspection/control records MUST remain distinct from task/trace/artifact exact runtime records.

## Invariants

- Daemon store remains canonical for task/trace/artifact/approval/coordination runtime state.
- Hazel exact records remain separate from daemon runtime task records even when both are inspectable through the daemon.
- Safe Hazel actions never bypass Hazel validators or policy checks.
- Retrieval preview, validation, submission, and sleep-job actions remain bounded and auditable.
- Sleep jobs remain proposal-producing only and do not perform opaque direct canonical writes.
- TUI-visible Hazel screens are derived views over daemon Hazel control-plane responses.

## Task Contracts

### Task 1: Define daemon Hazel inspection/read surfaces

**Preconditions**

- `docs/specs/hazel-structured-memory.md` remains the active subsystem spec for Hazel core behavior.

**Invariants**

- The daemon remains the sole Hazel authority exposed to clients.
- Hazel inspection stays distinct from task execution and MCP server management.

**Postconditions**

- The spec defines daemon-returned Hazel status, card views, proposal-batch views, and sleep-job views.

**Tests (must exist before implementation)**

Unit:
- `hazel_protocol_status_response_is_bounded`
- `hazel_protocol_card_view_preserves_provenance_fields`

Invariant:
- `hazel_inspection_surfaces_never_bypass_daemon_authority`

Integration:
- `hazel_list_cards_returns_transport_safe_view`
- `hazel_get_proposal_batch_returns_exact_provenance_summary`

Property-based (optional):
- `hazel_card_listing_order_is_deterministic_for_same_store_state`

### Task 2: Define daemon Hazel safe control actions

**Preconditions**

- Hazel validation/submission and sleep-job constraints remain active from the Hazel subsystem spec.

**Invariants**

- No control action performs opaque direct canonical-memory mutation.
- Validation remains explicit and fail-closed.

**Postconditions**

- The spec defines retrieval preview, proposal validation, proposal submission, and sleep-job enqueue/list/get/cancel contracts.

**Tests (must exist before implementation)**

Unit:
- `hazel_preview_request_rejects_oversized_limits`
- `hazel_validate_batch_rejects_unknown_policy_ids_in_strict_mode`

Invariant:
- `hazel_sleep_jobs_remain_proposal_producing_only`

Integration:
- `hazel_preview_returns_derived_payload_without_canonical_write`
- `hazel_submit_batch_persists_submission_outcome_record`
- `hazel_cancel_sleep_job_stops_future_proposal_production`

Property-based (optional):
- `hazel_preview_for_same_inputs_is_deterministic`

### Task 3: Define TUI and CLI daemon-client expectations

**Preconditions**

- Chat-first TUI and interactive event loop control-plane patterns remain active.

**Invariants**

- TUI and CLI remain daemon clients, not alternate Hazel authorities.
- Hazel UI surfaces remain distinct from task/chat surfaces.

**Postconditions**

- The spec defines Hazel screen/command expectations over daemon control-plane APIs.

**Tests (must exist before implementation)**

Unit:
- `hazel_tui_screen_renders_daemon_returned_status_and_counts`

Invariant:
- `hazel_tui_views_remain_derived_from_daemon_records`

Integration:
- `hazel_slash_commands_dispatch_only_through_daemon_control_plane`
- `hazel_cli_commands_mirror_daemon_control_plane_contracts`

Property-based (optional):
- not applicable

## Verification

- `scripts/doc-lint.sh --path docs/specs/hazel-inspection-tooling.md --strict-new`
- `scripts/check-tasks-registry.sh`
- `scripts/check-tasks-sync.sh --changed`
