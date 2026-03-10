# Hazel Structured Memory Subsystem

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Define tests before implementation work.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Invariant, and Integration checks.
> - Use Property-based tests only when a generative framework is actually used.
> - Run `scripts/doc-lint.sh` before commit.

Updated: 2026-03-10
Status: active
Owner: runtime
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-HAZEL-MEMORY-SPEC-001, TASK-HAZEL-MEMORY-PLAN-001

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. This spec's task contracts and invariants.
4. In-task updates recorded explicitly in this document.

## Output Contract

- Preserve exact section headings in this template unless intentionally revised.
- Keep claims concrete and tied to observable evidence.
- Avoid introducing unstated requirements or hidden assumptions.

## Evidence / Verification Contract

- Every completion claim must cite verification commands/results in `## Verification`.
- Conflicting evidence must be called out explicitly before task closure.
- If verification cannot run, record why and the residual risk.

## Model Compatibility Notes

- XML-style delimiter blocks (e.g. `<context>`, `<constraints>`) are optional structure aids.
- Critical constraints must also be restated in plain language.
- This fallback is required for cross-model robustness (including GPT-5.3 behavior).

## Purpose

Define a Rust-first Hazel structured memory subsystem for Sharo with:
- deterministic canonical memory behavior
- strict proposal/acceptance separation
- strict structural MCP hook contracts
- deterministic pre-prompt memory augmentation

This spec keeps the current daemon persistence layer as-is and introduces Hazel as an additive subsystem.

## Scope

### In Scope

- Hazel canonical domain model and lifecycle semantics.
- Proposal batch acceptance and deterministic resolution/merge/promotion logic.
- Recollection output contract for prompt-time memory augmentation.
- Hook contract for pre-prompt composition (`pre_prompt_compose`).
- Strict structural schema checks for configured MCP bindings.
- Composition policy baseline (`single`) with forward-compatible reducer model.
- Strict failure behavior for contract mismatch and invalid outputs.
- Conversation-log import interfaces that normalize external transcript formats into Hazel proposal batches.
- Bulk proposal submission interfaces for external extraction pipelines (for example paper/book processing).
- Sleep-job orchestration contract as proposal-producing work (not direct canonical mutation).
- Crate-boundary contract for Hazel components and future extension crates.

### Out of Scope

- Replacing daemon canonical task/trace/artifact persistence.
- Implementing the separate file-based personal journal memory subsystem.
- Multi-binding composition runtime behavior beyond explicit v1 rejection.
- Generic plugin runtime or arbitrary user-defined code execution for reducers.
- Implementing production-grade external extraction pipelines (LLM/rule hybrids) inside Hazel core.
- Implementing broad connector-specific parsers for every external conversation format in initial rollout.
- Finalizing specialized external ingestion binaries/crates (for example `hazel-cli`) in this phase.

## Core Terms

- `Hazel`: structured memory subsystem for typed, provenance-linked memory records.
- `Proposal`: explicit candidate write input to Hazel; may be inferentially produced upstream.
- `Canonical Core`: deterministic Hazel acceptance/merge/score/state-transition engine.
- `Recollection`: derived prompt-usable memory payload produced from canonical records.
- `WireInput` / `WireOutput`: JSON payloads exchanged with MCP tools.
- `BindingCanonicalOutput`: normalized per-binding Rust output type.
- `HookCanonicalOutput`: normalized hook-level Rust output type consumed by runtime.
- `Structural Compatibility`: schema-subtyping compatibility check between expected and configured tool schemas.
- `Composition Policy`: deterministic policy for handling one or more binding outputs.
- `Policy ID`: stable identifier for a runtime-managed instruction policy bundle (for example `hunch.v1`).
- `Policy Registry`: runtime configuration map from `Policy ID` to additive policy rules.
- `Conversation Import Adapter`: format-specific translator that converts logs/events into Hazel `ProposalBatch`.
- `Bulk Proposal Ingestion`: acceptance of large proposal sets produced outside Hazel canonical core.
- `Sleep Job`: asynchronous/background Hazel workflow that outputs proposal batches for deterministic acceptance.

## Crate Scope Boundaries

- `sharo-core` (in scope now):
  - canonical hook contract types shared by daemon/clients
  - canonical recollection payload types and validation-facing data model
  - policy compilation input/output contract types used across runtime boundaries
- `sharo-hazel-core` (in scope now):
  - canonical domain types
  - deterministic acceptance/resolution/merge/lifecycle
  - retrieval/recollection selection contracts
  - proposal batch validation and deterministic processing
- `sharo-hazel-mcp` (in scope now):
  - stdio MCP server/wrapper for Hazel operations
  - structural schema exposure and validation plumbing
  - wire-to-canonical normalization for hook and ingestion endpoints
- `sharo-daemon` integration (in scope now):
  - pre-prompt hook execution (`pre_prompt_compose`)
  - strict contract gating and policy compilation
  - injection of canonical recollection payloads
- Future crate candidates (out of scope in this phase):
  - `sharo-hazel-cli` or `sharo-hazel-ingest`: dedicated external ingestion tools (conversation files, papers/books, batch pipelines)
  - `sharo-hazel-sleep`: dedicated scheduler/worker runtime if sleep orchestration outgrows daemon-hosted job control

## Interfaces / Contracts

- Hazel integration hook point is `pre_prompt_compose`.
- Runtime MUST support 0 or 1 configured binding for `pre_prompt_compose` in v1.
- If more than one binding is configured for the hook, startup validation MUST fail.
- Hazel crates MUST synchronize shared dependency constraints with existing Sharo workspace crates for common libraries (for example `serde`, `serde_json`, `tokio`, `clap` where applicable).
- New Hazel crates MUST avoid introducing competing major versions of dependencies already present in the workspace unless explicitly approved by updated spec and plan artifacts.
- Dependency constraint drift for shared libraries MUST be treated as a policy violation during verification.
- Hook I/O contracts are structural, not nominal:
  - input compatibility: expected hook input schema must be accepted by bound tool input schema
  - output compatibility: bound tool output schema must be accepted by expected hook output schema
- Runtime MUST validate every request and response payload against the negotiated schemas.
- Pre-prompt retrieval knobs MUST be configurable and transmitted as hook input:
  - `top_k`
  - `token_budget`
  - `relevance_threshold`
- Runtime MUST run semantic lint after schema validation and before prompt injection:
  - required provenance per card
  - allowed memory states only
  - bounded `k` and token budget
  - bounded payload byte size
  - bounded card count
- Unknown fields in canonical payload sections MUST be rejected.
- Canonical writes MUST be mechanical and deterministic; no opaque LLM-mediated canonical writes.
- Proposal generation may be inferential, but proposal acceptance/merge/state transitions MUST be deterministic.
- Recollection payload MUST include memory state and uncertainty/provenance fields so the model can reason about confidence.
- Recollection payload MAY include `policy_ids` as an additive list; Hazel MUST NOT embed free-form behavioral rule text as memory content.
- Runtime MUST resolve `policy_ids` through a local policy registry (manifest/config), not from model-visible memory text.
- Policy application order MUST be deterministic and stable.
- Effective policy merge MUST be additive and monotonic:
  - stricter rule wins on conflict
  - no policy may relax an already stronger active rule unless explicitly permitted by a higher-priority runtime policy layer
- Unknown `policy_id` behavior MUST be explicit and default to fail-closed in strict mode.
- Hazel MUST preserve references to exact records; it MUST NOT replace or mutate daemon exact records as canonical runtime truth.
- Hazel MUST expose a canonical conversation import interface that maps normalized external logs to proposal batches:
  - `import_conversation_log(format, source_ref, payload, options) -> ProposalBatch|ProposalBatchSet`
- Hazel MUST expose bulk proposal submission and validation surfaces:
  - `validate_proposal_batch`
  - `submit_proposal_batch`
  - `submit_proposal_batches`
- Bulk ingestion inputs MUST support idempotency keys and per-batch provenance metadata.
- Hazel sleep orchestration MUST be proposal-producing only:
  - sleep jobs may create proposal batches
  - sleep jobs MUST NOT perform opaque direct canonical writes
- Sleep execution contract MUST support bounded budgets (time/work units), deterministic run identifiers, and idempotent retry behavior.
- Daemon hook observability MUST emit operator-visible success/failure events for pre-prompt tool execution including:
  - binding id
  - task id
  - elapsed time
  - failure reason (when applicable)

## Daemon Config Hook Surface (v1)

- `reasoning_hooks.pre_prompt_compose.bindings[]`:
  - `id`
  - `tool`
  - `command`
  - `args` (optional)
  - `timeout_ms` (optional, defaults in runtime)
- `reasoning_hooks.pre_prompt_compose`:
  - `composition`
  - `default_policy_ids`
  - `strict_unknown_policy_ids`
  - `top_k` (optional)
  - `token_budget` (optional)
  - `relevance_threshold` (optional)
- `hazel_manifest.cards[]`:
  - `kind`
  - `policy_ids`
  - `max_cards` (optional)

## Invariants

- Current daemon store remains canonical for exact runtime records (`Task`, `Trace`, `Artifact`, approvals, coordination).
- Hazel core does not infer from raw text autonomously during canonical acceptance.
- `Assertion` lineage is append-only; derived assertions preserve source assertion lineage.
- `Association` does not imply `Relation`; co-activation may strengthen associations but does not create semantic relations.
- Retrieval/recollection preserves explicit uncertainty; candidate/contested records remain first-class and visible.
- Hook composition behavior is deterministic, explicit, and policy-driven; no implicit merge behavior.
- Structural contract mismatch is fail-fast and operator-visible.
- Invalid MCP output never reaches prompt injection.
- Shared dependency constraints remain aligned with the workspace baseline for common crates.
- Policy behavior is runtime-governed via registry mapping; memory records do not carry mutable policy rule bodies.

## Task Contracts

### Task 1: Define Hazel canonical data contracts and deterministic lifecycle

**Preconditions**

- Existing MVP memory and persistence invariants remain active.

**Invariants**

- Canonical Hazel records and transitions are deterministic.
- Proposal/acceptance split is explicit.
- File-based journal memory remains a separate subsystem.

**Postconditions**

- Hazel schema contracts are specified for `Chunk`, `Entity`, `Relation`, `Association`, `Assertion`, and `Activation`.
- State/scoring fields are specified with deterministic transition formulas.

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

### Task 2: Define strict structural MCP hook contract validation

**Preconditions**

- Hook points and expected hook schemas are defined in runtime config model.

**Invariants**

- Contract checks are structural and schema-based.
- Runtime validates both request and response payloads.
- Schema pass is necessary but not sufficient; semantic lint is still required.

**Postconditions**

- Startup rejects incompatible hook-tool bindings.
- Runtime rejects invalid or semantically non-compliant outputs before injection.

**Tests (must exist before implementation)**

Unit:
- `hook_binding_rejected_when_input_schema_incompatible`
- `hook_binding_rejected_when_output_schema_incompatible`
- `hook_runtime_rejects_response_missing_provenance`

Invariant:
- `hook_runtime_never_injects_unvalidated_mcp_payload`

Integration:
- `pre_prompt_compose_accepts_structurally_compatible_hazel_binding`

Property-based (optional):
- not applicable

### Task 3: Define hook composition policy baseline and extension contract

**Preconditions**

- Hook config parser and binding metadata model exist.

**Invariants**

- v1 allows only `single` composition policy.
- Multi-binding config for a single hook is rejected in v1.
- Future composition is defined over canonicalized outputs, never raw wire payloads.
- Recollection policies are additive by `policy_ids`, with deterministic merge order and strict conflict semantics.

**Postconditions**

- `single` policy behavior is fully specified.
- Extension model for future reducers is specified without enabling them in v1.
- Additive `policy_ids` composition and runtime registry mapping are fully specified for v1 single-binding flow.

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

### Task 4: Define ingestion and sleep orchestration contracts with crate boundaries

**Preconditions**

- Canonical Hazel proposal contracts are defined.

**Invariants**

- External extraction logic remains outside Hazel canonical core.
- Import and sleep paths produce proposals, not opaque canonical writes.
- Crate boundaries are explicit for current phase versus future crate candidates.

**Postconditions**

- Conversation-log import interfaces are specified with supported baseline formats and adapter contract.
- Bulk proposal ingestion contract is specified with idempotency/provenance requirements.
- Sleep-job orchestration contract is specified as bounded, replayable proposal-producing workflow.
- Future crate considerations (`hazel-cli`/specialized ingestion and sleep workers) are documented as explicit follow-up decisions.

**Tests (must exist before implementation)**

Unit:
- `conversation_import_adapter_maps_openai_messages_to_proposal_batch`
- `bulk_submit_requires_idempotency_key_and_batch_provenance`
- `sleep_job_output_must_be_proposal_batches_only`

Invariant:
- `sleep_path_never_performs_unattributed_direct_canonical_mutation`

Integration:
- `paper_ingest_pipeline_submits_bulk_batches_with_resolution_traces`

Property-based (optional):
- `bulk_ingestion_order_does_not_change_deterministic_acceptance_result`

## Scenarios

- S1: User asks architecture question; runtime prefetches Hazel recollection before model turn and injects bounded, provenance-rich cards.
- S2: Configured MCP binding advertises incompatible output schema; startup fails fast with explicit error.
- S3: Binding returns schema-valid payload with missing provenance; runtime rejects payload and emits a visible diagnostic event.
- S4: Operator configures two bindings on `pre_prompt_compose`; runtime rejects startup under v1 `single` policy.
- S5: Recollection includes `candidate` and `contested` items; prompt payload keeps uncertainty explicit rather than flattening into asserted facts.
- S6: Recollection carries `policy_ids=["hunch.v1","safety.strict.v1"]`; runtime deterministically composes effective rules with stricter constraints preserved.
- S7: Conversation log import from OpenAI-style `messages[]` produces chunks and candidates via adapter, then deterministic canonical acceptance.
- S8: External paper-processing pipeline submits bulk proposal batches with idempotency keys; Hazel returns deterministic per-candidate resolution traces.
- S9: Sleep run performs alias/association maintenance by emitting proposal batches only; no direct canonical mutation path exists.

## Verification

- `scripts/doc-lint.sh --changed --strict-new`
- `scripts/check-tasks-registry.sh`
- `scripts/check-tasks-sync.sh --changed`

## Risks and Failure Modes

- Overly strict schema boundaries can reject useful provider payloads unless extension channels are explicitly modeled.
- Output-schema metadata is not always available from MCP providers; sidecar schema manifests may be required.
- Fail-closed startup policy can reduce availability if providers drift; operational rollout must include version pinning and staged updates.
- Prompt budget overflow risk remains if recollection summarization limits are misconfigured.
- Policy registry drift between environments can cause behavior divergence if policy bundles are not pinned and verified.
- Ingestion adapters can drift across providers/formats; strict adapter contracts and fixture tests are required to prevent silent semantic skew.
- Sleep backlog growth can affect timeliness; bounded budgets and explicit scheduling policy are required.

## Open Questions

- Should strict mode fail the whole daemon startup or only disable the failing hook binding?
- Should schema compatibility allow minor-version widening under explicit config opt-in?
- What is the minimum required recollection card shape for first production rollout?
- Should unknown `policy_id` ever be allowed under explicit non-strict development mode?
- Which conversation formats are mandatory in v1 versus optional adapters?
- Should sleep orchestration remain daemon-hosted initially or move early to a dedicated Hazel worker crate?

## References

- [mvp.md](/home/dikini/Projects/sharo/docs/specs/mvp.md)
- [store-transactional-persistence.md](/home/dikini/Projects/sharo/docs/specs/store-transactional-persistence.md)
- [store-directory-fsync-commit-consistency.md](/home/dikini/Projects/sharo/docs/specs/store-directory-fsync-commit-consistency.md)
