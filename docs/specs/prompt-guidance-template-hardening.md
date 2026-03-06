# Prompt Guidance Template Hardening

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Define tests before implementation work.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Invariant, and Integration checks.
> - Use Property-based tests only when a generative framework is actually used.
> - Run `scripts/doc-lint.sh` before commit.

Updated: 2026-03-06
Status: active
Owner: platform
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-PROMPT-GUIDANCE-SPEC-001, TASK-PROMPT-GUIDANCE-PLAN-001

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and repository workflow policies.
3. This spec's contracts and invariants.
4. Explicit in-task updates recorded in this document.

## Output Contract

- Keep section headings and contracts explicit and unambiguous.
- State assumptions and conflicts directly.
- Avoid implicit behavior changes not covered by task contracts.

## Evidence / Verification Contract

- Verification commands listed in `## Verification` are required before completion claims.
- If a check cannot run, record the reason and residual risk.
- Contradictory evidence must be resolved or explicitly escalated.

## Model Compatibility Notes

- XML-style delimiter blocks are optional structure aids, not parser requirements.
- Critical constraints must also be repeated in plain language.
- This fallback is required for robust behavior on GPT-5.3 as well as GPT-5.4.

## Purpose

Strengthen project templates and agent-facing workflow guidance using structured prompt contracts (explicit sections, output constraints, priority order, and verification loops) so plan/spec execution remains deterministic across model upgrades and model variants.

## Scope

### In Scope

- Add explicit prompt-contract sections to `docs/templates/spec.template.md` and `docs/templates/plan.template.md`.
- Add model-agnostic structured delimiter guidance (XML-style blocks as plain text conventions).
- Add compatibility rules for GPT-5.3 and GPT-5.4 behavior differences.
- Add machine checks and documentation updates to keep prompt-contract usage consistent.

### Out of Scope

- Runtime protocol or daemon behavior changes.
- Enforcing model-specific APIs at runtime.
- Rewriting historical specs/plans that predate this contract (migration is forward-looking).

## Core Terms

- `Prompt Contract`: deterministic structure describing required sections, output format, and success criteria.
- `Instruction Priority Ladder`: explicit ordering for conflicts (`system` > `developer` > task-local spec/plan).
- `Output Contract`: strict statement of required output shape and prohibited output.
- `Execution Loop Contract`: explicit policy for verify/fix/re-verify before claiming completion.
- `Delimiter Block`: XML-style or tagged text blocks used as readable structure in prompts (`<context>`, `<constraints>`, `<output_contract>`, etc.).

## Interfaces / Contracts

- `docs/templates/spec.template.md` must include:
  - `Instruction Priority`
  - `Output Contract`
  - `Evidence / Verification Contract`
  - `Model Compatibility Notes`
- `docs/templates/plan.template.md` must include:
  - `Execution Mode` (plan-only vs execute)
  - `Task Update Contract`
  - `Verification Gate` per task
  - `Completion Gate` for end-of-plan status
- `scripts/doc-lint.sh` and related tests must reject new spec/plan docs missing required prompt-contract sections.
- `docs/templates/README.md` must document acceptable delimiter-block usage and fallback phrasing.

## Invariants

- Template guidance remains model-agnostic and readable in plain Markdown.
- XML-style delimiter blocks are optional syntactic aids; semantics must still be clear without XML parsing support.
- Critical constraints are duplicated in plain language for GPT-5.3 robustness.
- No success claim is allowed without explicit verification evidence references.

## Task Contracts

### Task 1: Template Prompt Contract Augmentation

**Preconditions**

- Existing template files are available and doc-lint baseline passes.

**Invariants**

- Added sections remain concise and do not require proprietary tooling.

**Postconditions**

- New specs/plans generated from templates include deterministic prompt-contract sections by default.

**Tests (must exist before implementation)**

Unit:
- `test_spec_template_contains_output_contract_sections`
- `test_plan_template_contains_execution_and_completion_contract_sections`

Invariant:
- `test_doc_lint_rejects_new_docs_missing_prompt_contract_sections`

Integration:
- `test_doc_start_generates_prompt_contract_sections_for_spec_and_plan`

Property-based (optional):
- not applicable

### Task 2: Workflow Guidance Alignment

**Preconditions**

- Template prompt-contract sections are defined.

**Invariants**

- Workflow docs stay consistent with template contract language.

**Postconditions**

- Contributor guidance explicitly documents when and how to use delimiter blocks and fallback plain-language reinforcement.

**Tests (must exist before implementation)**

Unit:
- `test_templates_readme_documents_delimiter_block_and_plain_language_fallback`

Invariant:
- `test_doc_terms_accepts_prompt_contract_vocabulary`

Integration:
- `test_fast_feedback_remains_green_with_updated_template_docs`

Property-based (optional):
- not applicable

### Task 3: GPT-5.3 and GPT-5.4 Compatibility Policy

**Preconditions**

- Template and workflow sections are updated.

**Invariants**

- Contracts remain valid under both GPT-5.3 and GPT-5.4.

**Postconditions**

- Policy clearly states expected adherence differences and required fallback strategies for GPT-5.3.

**Tests (must exist before implementation)**

Unit:
- `test_prompt_guidance_spec_includes_model_compatibility_clause`

Invariant:
- `test_plan_template_requires_plain_language_restatement_for_critical_constraints`

Integration:
- `test_example_plan_renders_valid_contract_without_xml_tags`

Property-based (optional):
- not applicable

## Scenarios

- S1: Contributor creates a new plan and forgets output-shape requirements; template now forces explicit output contract.
- S2: Agent uses XML-style blocks on GPT-5.3 and partially ignores one block; plain-language duplicate constraints preserve intent.
- S3: Agent claims completion without verification evidence; completion gate blocks this in template workflow.
- S4: Mid-task instruction change occurs; task update contract preserves deterministic priority and next-step behavior.

## Verification

- `scripts/doc-lint.sh --changed --strict-new`
- `scripts/check-doc-terms.sh --changed`
- `scripts/check-tasks-registry.sh`
- `scripts/check-tasks-sync.sh --changed`

## Risks and Failure Modes

- Overly strict contract sections can increase authoring friction for short docs.
- Excess delimiter markup can reduce readability if not kept concise.
- Compatibility notes can drift as model behavior changes; periodic review is required.

## Open Questions

- Should prompt-contract sections be mandatory only for new files, or also backfilled in high-churn existing plans/specs?
- Should the doc-lint gate enforce exact heading names or accept a synonym map?

## References

- OpenAI prompt guidance: <https://developers.openai.com/api/docs/guides/prompt-guidance>
- [deterministic-workflow-hardening.md](/home/dikini/Projects/sharo/docs/specs/deterministic-workflow-hardening.md)
- [workflow-tool-guides.md](/home/dikini/Projects/sharo/docs/specs/workflow-tool-guides.md)
