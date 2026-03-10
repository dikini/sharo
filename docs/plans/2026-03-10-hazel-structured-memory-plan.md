# Hazel Structured Memory Implementation Plan

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Invariant, and Integration checks.
> - Use Property-based tests only when a generative framework is actually used.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: Implement Hazel as a strict structured memory subsystem with deterministic core behavior and strict structural MCP hook validation for pre-prompt recollection injection.
Architecture: Keep existing daemon persistence as canonical exact-runtime storage while adding Hazel as an additive subsystem. Introduce one core crate for canonical structured-memory logic and one MCP wrapper crate for stdio server/client integration. Bind Hazel to runtime via `pre_prompt_compose` hook under strict schema validation and v1 single-binding composition, with additive `policy_ids` resolved through runtime policy registry mapping.
Tech Stack: Rust 2024, serde/serde_json, JSON Schema validation crate, sharo-daemon hook pipeline, stdio MCP transport.
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-HAZEL-MEMORY-SPEC-001, TASK-HAZEL-MEMORY-PLAN-001

Implementation Update (2026-03-10):
- Implemented baseline ingestion and sleep interfaces directly in `sharo-hazel-core` as deterministic contracts and validators.
- External extractor/connector tooling remains deferred to future specialized crates (`sharo-hazel-cli` / `sharo-hazel-ingest`), consistent with scope boundaries.
- Implemented daemon pre-prompt stdio hook execution for Hazel with strict shared-contract validation and fail-closed behavior.
- Centralized hook schema/value contracts in `sharo-core` and aligned both `sharo-daemon` and `sharo-hazel-mcp` to those contracts.
- Implemented manifest-driven card policy hints (`hazel_manifest.cards`) with additive deterministic policy resolution.
- Implemented deterministic retrieval in `sharo-hazel-core` and wired `sharo-hazel-mcp` to use it for `hazel.recollect`.
- Added daemon test coverage that invokes the actual `sharo-hazel-mcp` binary (not only mock scripts).
- Added configurable retrieval knobs in pre-prompt hook flow (`top_k`, `token_budget`, `relevance_threshold`) and daemon metadata emission.
- Decision status: keep dedicated `sharo-hazel-cli` / `sharo-hazel-ingest` as future work; current phase keeps ingestion interfaces in `sharo-hazel-core` and daemon-side orchestration.

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. This plan's execution and verification contracts.
4. Explicit plan updates recorded in this file.

## Execution Mode

- Mode: plan-only
- Default: execute-with-checkpoints unless user explicitly requests plan-only output.

## Output Contract

- Keep task definitions concrete: exact files, commands, and expected outcomes.
- Use Red/Green checkpoints as hard gates before claiming task completion.
- Record unresolved risks instead of silently skipping checks.
- Keep Hazel crate dependency constraints synchronized with existing workspace constraints for shared libraries.

## Task Update Contract

- New instructions must be mapped to affected tasks before continuing execution.
- If priority conflicts exist, apply Instruction Priority and document the resolution.
- Do not silently drop prior accepted requirements.

## Completion Gate

- A task is complete only when Preconditions, Invariants, Postconditions, and Tests are all satisfied.
- Plan completion requires explicit verification evidence and changelog/task-registry alignment.

## Model Compatibility Notes

- XML-style delimiter blocks are optional and treated as plain-text structure.
- Critical constraints must be restated in plain language for model-robust adherence.

## Crate Scope Matrix

- In scope now:
  - `sharo-core`: shared hook contract types and canonical recollection payload schemas.
  - `sharo-hazel-core`: canonical model, deterministic acceptance/lifecycle, retrieval contracts.
  - `sharo-hazel-mcp`: stdio MCP wrapper, schema contracts, wire normalization.
  - `sharo-daemon` integration touchpoints: strict pre-prompt hook pipeline and policy compilation.
- Future work:
  - specialized ingestion crate (for example `sharo-hazel-cli` or `sharo-hazel-ingest`) for conversation logs and document pipelines.
  - dedicated sleep worker crate (for example `sharo-hazel-sleep`) if orchestration scale exceeds daemon-hosted job control.

---

### Task 1: Add Hazel core crate contracts and deterministic canonical model

**Files:**

- Create:
  - `crates/sharo-hazel-core/Cargo.toml`
  - `crates/sharo-hazel-core/src/lib.rs`
  - `crates/sharo-hazel-core/src/domain.rs`
  - `crates/sharo-hazel-core/src/proposal.rs`
  - `crates/sharo-hazel-core/src/lifecycle.rs`
  - `crates/sharo-hazel-core/tests/domain_contracts.rs`
- Modify:
  - `Cargo.toml`
- Test:
  - `crates/sharo-hazel-core/tests/domain_contracts.rs`

**Preconditions**

- Hazel structured-memory spec is active and task registry references are present.

**Invariants**

- Canonical Hazel core logic remains mechanical and deterministic.
- Proposal generation logic remains outside canonical acceptance logic.
- Shared dependency constraints for common crates remain aligned with existing workspace crates.

**Postconditions**

- Hazel core crate exposes typed domain and lifecycle contracts for canonical memory model.
- Unknown-field rejection and lineage-preservation contracts are represented in tests.

**Tests (must exist before implementation)**

Unit:
- `hazel_core_rejects_unknown_fields_in_canonical_sections`
- `hazel_core_preserves_assertion_lineage_on_derived_assertions`

Invariant:
- `hazel_core_association_does_not_imply_relation`

Integration:
- `hazel_core_candidate_to_active_transition_is_formula_driven`

Property-based (optional):
- `hazel_core_deterministic_resolution_is_order_stable`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-hazel-core`
Expected: fails because crate/tests are not implemented yet

**Implementation Steps**

1. Add `sharo-hazel-core` crate to workspace with typed contracts and strict serde behavior.
2. Synchronize shared dependency constraints with existing workspace crates (either via `[workspace.dependencies]` or exact aligned crate constraints).
3. Implement deterministic lifecycle/state scaffolding with explicit transition helpers.
4. Add tests for strict schema/lineage/state invariants.

**Green Phase (required)**

Command: `cargo test -p sharo-hazel-core`
Expected: all Hazel core tests pass

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/sharo-hazel-core/**`
Re-run: `cargo test -p sharo-hazel-core`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

---

### Task 2: Add Hazel MCP crate with strict structural wire contract surfaces

**Files:**

- Create:
  - `crates/sharo-hazel-mcp/Cargo.toml`
  - `crates/sharo-hazel-mcp/src/main.rs`
  - `crates/sharo-hazel-mcp/src/schema.rs`
  - `crates/sharo-hazel-mcp/src/normalize.rs`
  - `crates/sharo-hazel-mcp/tests/schema_contracts.rs`
- Modify:
  - `Cargo.toml`
- Test:
  - `crates/sharo-hazel-mcp/tests/schema_contracts.rs`

**Preconditions**

- Hazel core crate contracts exist.

**Invariants**

- Wire schemas are explicit and versioned.
- Runtime-facing canonical outputs are produced only after schema and semantic validation.
- Shared dependency constraints for common crates remain aligned with existing workspace crates.
- Wire output supports `policy_ids` metadata as identifiers only; no free-form rule bodies are accepted from memory payloads.

**Postconditions**

- Hazel MCP crate exposes stdio server wrapper with strict input/output schema metadata.
- Structural compatibility checks are executable from Rust tests.
- Canonical output normalization preserves `policy_ids` as typed identifiers for runtime policy resolution.

**Tests (must exist before implementation)**

Unit:
- `hook_binding_rejected_when_input_schema_incompatible`
- `hook_binding_rejected_when_output_schema_incompatible`
- `hook_runtime_rejects_response_missing_provenance`
- `hook_runtime_rejects_rule_text_payload_when_only_policy_ids_allowed`

Invariant:
- `hook_runtime_never_injects_unvalidated_mcp_payload`

Integration:
- `pre_prompt_compose_accepts_structurally_compatible_hazel_binding`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-hazel-mcp`
Expected: fails because crate/tests are not implemented yet

**Implementation Steps**

1. Add stdio MCP crate with explicit wire schema definitions and canonical normalization layer.
2. Synchronize shared dependency constraints with existing workspace crates (including common runtime and serialization crates).
3. Implement structural schema compatibility checker and semantic lint pass.
4. Add tests for compatibility success/failure and validation gating behavior.
5. Ensure output normalization enforces `policy_ids`-only policy metadata contract.

**Green Phase (required)**

Command: `cargo test -p sharo-hazel-mcp`
Expected: all Hazel MCP tests pass

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/sharo-hazel-mcp/**`
Re-run: `cargo test -p sharo-hazel-mcp`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

---

### Task 3: Integrate strict hook policy in daemon pre-prompt flow (single-binding v1)

**Files:**

- Modify:
  - `crates/sharo-core/src/protocol.rs`
  - `crates/sharo-core/src/context_resolvers.rs`
  - `crates/sharo-daemon/src/config.rs`
  - `crates/sharo-daemon/src/kernel.rs`
  - `crates/sharo-daemon/tests/scenario_a.rs`
  - `crates/sharo-daemon/tests/daemon_ipc.rs`
- Test:
  - `crates/sharo-daemon/tests/scenario_a.rs`
  - `crates/sharo-daemon/tests/daemon_ipc.rs`

**Preconditions**

- Hazel core and Hazel MCP contract layers exist.

**Invariants**

- Existing daemon store canonical persistence behavior is unchanged.
- Hook payloads are validated before use.
- v1 hook composition rejects more than one binding for `pre_prompt_compose`.
- Any new daemon-side dependencies introduced for hook execution stay aligned with workspace constraints for shared crates.
- Runtime resolves additive `policy_ids` through local policy registry with deterministic, monotonic merge semantics.

**Postconditions**

- Runtime supports pre-prompt Hazel recollection injection through strict hook interface.
- Invalid binding config and invalid payload behaviors are deterministic and visible.
- Runtime composes effective instruction bundle from additive `policy_ids` and injects compiled control payload.
- Hook and recollection payload contract types are anchored in `sharo-core` rather than daemon-local types.

**Tests (must exist before implementation)**

Unit:
- `hook_policy_single_rejects_multiple_bindings`
- `hook_policy_single_accepts_zero_or_one_binding`
- `hook_policy_registry_rejects_unknown_policy_id_in_strict_mode`
- `hook_policy_merge_prefers_stricter_constraint`

Invariant:
- `hook_composition_never_merges_raw_wire_outputs`
- `hook_policy_merge_is_deterministic_for_same_policy_set`

Integration:
- `pre_prompt_compose_single_binding_injects_canonical_recollection_payload`
- `pre_prompt_compose_resolves_policy_ids_to_effective_instruction_bundle`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-daemon hook_policy_single_rejects_multiple_bindings -- --exact`
Expected: fails because hook policy checks are not implemented yet

**Implementation Steps**

1. Add canonical hook/recollection contract types in `sharo-core`.
2. Add hook config model and startup validation for structural compatibility and single-binding policy.
3. Add runtime policy registry config model for additive `policy_ids` and strict unknown-policy handling.
4. Add deterministic policy compilation/merge from `policy_ids` to effective instruction bundle.
5. Add pre-prompt hook execution path and canonical injection into memory resolver output.
6. Add tests for startup rejection, runtime validation, policy resolution, and successful injection.

**Green Phase (required)**

Command: `cargo test -p sharo-daemon`
Expected: daemon tests pass including new hook policy and injection scenarios

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/sharo-daemon/src/config.rs`, `crates/sharo-daemon/src/kernel.rs`, `crates/sharo-core/src/context_resolvers.rs`
Re-run: `cargo test -p sharo-daemon`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

---

### Task 4: Define ingestion interfaces and baseline adapters without embedding extraction logic in core

**Files:**

- Modify:
  - `docs/specs/hazel-structured-memory.md`
  - `docs/plans/2026-03-10-hazel-structured-memory-plan.md`
- Future-create targets (not implemented in this phase):
  - `crates/sharo-hazel-cli/` (or `crates/sharo-hazel-ingest/`) for external conversation/document ingestion tooling
- Test:
  - adapter and ingestion contract tests in future implementation phase

**Preconditions**

- Core proposal contracts are defined.

**Invariants**

- Hazel canonical core remains mechanical and deterministic.
- Conversation/document extraction remains external to canonical acceptance logic.
- Ingestion contracts enforce idempotency and provenance.

**Postconditions**

- Interface-level contract for conversation import and bulk batch ingestion is documented.
- Baseline expected formats and adapter boundary are documented.
- Future specialized crate decision is explicitly recorded.

**Tests (must exist before implementation)**

Unit:
- `conversation_import_adapter_maps_openai_messages_to_proposal_batch`
- `bulk_submit_requires_idempotency_key_and_batch_provenance`

Invariant:
- `ingestion_path_never_bypasses_proposal_acceptance_gate`

Integration:
- `paper_ingest_pipeline_submits_bulk_batches_with_resolution_traces`

Property-based (optional):
- `bulk_ingestion_order_does_not_change_deterministic_acceptance_result`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-hazel-core conversation_import_adapter_maps_openai_messages_to_proposal_batch -- --exact`
Expected: fails until ingestion adapter contracts and implementations are added in future phase

**Implementation Steps**

1. Keep this phase doc-level for ingestion boundaries and crate decisions.
2. Defer executable adapters to dedicated future crate decision (`hazel-cli`/`hazel-ingest`).
3. Ensure planned adapter outputs are constrained to Hazel proposal batch contracts.

**Green Phase (required)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: docs pass with explicit ingestion boundary contracts

**Refactor Phase (optional but controlled)**

Allowed scope: `docs/specs/hazel-structured-memory.md`, `docs/plans/2026-03-10-hazel-structured-memory-plan.md`
Re-run: `scripts/doc-lint.sh --changed --strict-new`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks defined for future implementation
- CHANGELOG.md updated

---

### Task 5: Define sleep orchestration contract as proposal-producing workflow

**Files:**

- Modify:
  - `docs/specs/hazel-structured-memory.md`
  - `docs/plans/2026-03-10-hazel-structured-memory-plan.md`
- Future-create targets (not implemented in this phase):
  - optional `crates/sharo-hazel-sleep/` if dedicated worker runtime is selected
- Test:
  - sleep orchestration contract tests in future implementation phase

**Preconditions**

- Core proposal contracts and lifecycle rules are defined.

**Invariants**

- Sleep jobs never perform opaque direct canonical writes.
- Sleep workflows are bounded, replayable, and idempotent.

**Postconditions**

- Sleep job contract and job classes are documented.
- Daemon-hosted versus dedicated sleep-worker crate decision is explicitly deferred with criteria.

**Tests (must exist before implementation)**

Unit:
- `sleep_job_output_must_be_proposal_batches_only`
- `sleep_job_requires_bounded_budget_configuration`

Invariant:
- `sleep_path_never_performs_unattributed_direct_canonical_mutation`

Integration:
- `sleep_alias_consolidation_job_emits_proposals_and_preserves_lineage`

Property-based (optional):
- `sleep_retry_with_same_run_id_is_idempotent`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-hazel-core sleep_job_output_must_be_proposal_batches_only -- --exact`
Expected: fails until sleep orchestration contracts and implementation exist in future phase

**Implementation Steps**

1. Keep this phase doc-level for sleep orchestration boundaries and future crate considerations.
2. Define explicit criteria for moving from daemon-hosted scheduler to dedicated sleep worker crate.
3. Ensure sleep outputs are constrained to proposal batch submission paths.

**Green Phase (required)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: docs pass with explicit sleep orchestration boundary contracts

**Refactor Phase (optional but controlled)**

Allowed scope: `docs/specs/hazel-structured-memory.md`, `docs/plans/2026-03-10-hazel-structured-memory-plan.md`
Re-run: `scripts/doc-lint.sh --changed --strict-new`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks defined for future implementation
- CHANGELOG.md updated

---

### Task 6: Add adversarial fuzz/property coverage for MCP wire framing and schema contracts

**Files:**

- Modify:
  - `crates/sharo-core/tests/protocol_tests.rs`
  - `crates/sharo-hazel-mcp/src/main.rs`
  - `crates/sharo-hazel-mcp/tests/schema_contracts.rs`
- Future-create targets (decision in implementation phase):
  - `crates/sharo-hazel-mcp/fuzz/Cargo.toml`
  - `crates/sharo-hazel-mcp/fuzz/fuzz_targets/wire_request_frame.rs`
  - `crates/sharo-core/tests/protocol_property_tests.rs` (if property suite split is needed)
- Test:
  - protocol/schema property checks in `sharo-core`
  - fuzz targets for `sharo-hazel-mcp` wire input handling

**Preconditions**

- Strict schema compatibility and MCP size-bounded input handling are implemented.

**Invariants**

- Adversarial wire input must never bypass schema validation or semantic lint.
- Oversized, malformed, and boundary-framed input must fail closed without unbounded allocation.
- Schema compatibility helpers must remain deterministic for the same inputs.

**Postconditions**

- Property-based tests cover schema compatibility invariants and malformed schema descriptors.
- Fuzzing targets cover MCP wire framing/parsing edge cases (oversized line, missing newline, invalid UTF-8, repeated boundary inputs).
- CI or documented local verification path exists for repeatable fuzz regression runs.

**Tests (must exist before implementation)**

Unit:
- `line_content_len_excludes_trailing_newline`
- `line_content_len_keeps_non_terminated_length`

Invariant:
- `schema_compatibility_rejects_malformed_tool_schema_definition`
- `hook_runtime_never_injects_unvalidated_mcp_payload`

Integration:
- `pre_prompt_compose_accepts_structurally_compatible_hazel_binding`

Property-based:
- `prop_input_schema_compatibility_is_deterministic_for_same_schemas`
- `prop_output_schema_compatibility_is_deterministic_for_same_schemas`
- `prop_object_schema_well_formed_rejects_required_not_in_allowed_when_strict`

Fuzz:
- `wire_request_frame` target for `sharo-hazel-mcp` stdio request loop and size-boundary behavior

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-core prop_input_schema_compatibility_is_deterministic_for_same_schemas`
Expected: fails until property tests are implemented

Command: `cargo fuzz run wire_request_frame`
Expected: target missing/fails until fuzz harness is implemented

**Implementation Steps**

1. Add property tests in `sharo-core` for schema helper determinism and well-formedness invariants.
2. Add `sharo-hazel-mcp` fuzz harness for wire input framing/parsing and oversize handling.
3. Add regression corpus seeds for boundary payload sizes and malformed frame patterns.
4. Document a bounded fuzz execution recipe for local/CI smoke runs.

**Green Phase (required)**

Command: `cargo test -p sharo-core`
Expected: protocol property tests pass consistently

Command: `cargo test -p sharo-hazel-mcp`
Expected: MCP unit/integration tests pass with no regressions

Command: `cargo fuzz run wire_request_frame -- -max_total_time=30`
Expected: fuzz target executes without crashes for bounded smoke window

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/sharo-core/tests/**`, `crates/sharo-hazel-mcp/**`
Re-run: `cargo test -p sharo-core && cargo test -p sharo-hazel-mcp`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Property/fuzz coverage added and runnable
- CHANGELOG.md updated
