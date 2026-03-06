# Prompt Guidance Template Hardening Implementation Plan

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Invariant, and Integration checks.
> - Use Property-based tests only when a generative framework is actually used.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: upgrade spec/plan templates and workflow guidance to enforce deterministic prompt contracts aligned with modern prompt-guidance patterns while preserving cross-model compatibility.
Architecture: implement in three slices: template schema changes, lint/test enforcement, and workflow documentation alignment. Keep XML-style delimiters optional and reinforce critical constraints in plain language for GPT-5.3 reliability.
Tech Stack: Markdown templates, Bash lint scripts, Bats tests, docs policy gates.
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-PROMPT-GUIDANCE-SPEC-001, TASK-PROMPT-GUIDANCE-PLAN-001

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and repository workflow policies.
3. This plan's execution and verification contracts.
4. Explicit plan updates logged in this file.

## Execution Mode

- Mode: execute-with-checkpoints
- If user requests planning-only mode, stop after producing/validating the plan artifact.

## Output Contract

- Each task must specify exact files, commands, and expected outcomes.
- Red/Green phases are hard gates before task completion.
- Final status must include verification evidence and unresolved risks.

## Task Update Contract

- New user instructions are mapped to impacted tasks before proceeding.
- Priority conflicts are resolved using Instruction Priority and documented.
- Accepted requirements are not silently dropped.

## Completion Gate

- A task is complete only after Preconditions, Invariants, Postconditions, and tests are satisfied.
- Plan completion requires successful verification commands plus changelog/task-registry updates where policy requires them.

## Model Compatibility Notes

- XML-style delimiter blocks are optional and treated as plain text.
- Critical constraints are always duplicated in plain language for GPT-5.3 compatibility.

---

### Task 1: Add Prompt-Contract Sections to Templates

**Files:**

- Modify: `docs/templates/spec.template.md`
- Modify: `docs/templates/plan.template.md`
- Modify: `docs/templates/README.md`
- Test: `scripts/tests/test-doc-tools.bats`

**Preconditions**

- Existing templates and doc tooling are passing.

**Invariants**

- Templates remain plain Markdown with no model-specific runtime dependencies.
- XML-style block usage remains optional and explanatory.

**Postconditions**

- Newly generated docs include explicit:
  - instruction priority
  - output contract
  - verification/completion contract
  - model-compatibility notes

**Tests (must exist before implementation)**

Unit:
- `test_spec_template_includes_prompt_contract_sections`
- `test_plan_template_includes_execution_completion_contract_sections`

Invariant:
- `test_templates_readme_includes_delimiter_block_guidance`

Integration:
- `test_doc_start_generated_files_include_prompt_contract_sections`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `bats scripts/tests/test-doc-tools.bats`
Expected: fails on missing prompt-contract assertions.

**Implementation Steps**

1. Add prompt-contract headings and guidance bullets to both templates.
2. Document delimiter-block conventions and plain-language fallback in template README.
3. Extend Bats doc-tooling tests to assert presence of new sections.

**Green Phase (required)**

Command: `bats scripts/tests/test-doc-tools.bats`
Expected: all updated template contract tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `docs/templates/*`, `scripts/tests/test-doc-tools.bats`
Re-run: `scripts/doc-lint.sh --changed --strict-new`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 2: Enforce Prompt-Contract Presence in Docs Lint Flow

**Files:**

- Modify: `scripts/doc-lint.sh`
- Modify: `scripts/tests/test-doc-tools.bats`
- Modify: `docs/tasks/README.md`

**Preconditions**

- Template sections are defined and stable.

**Invariants**

- Lint enforcement applies to newly created/changed strict-profile docs only.
- Error messages provide actionable guidance.

**Postconditions**

- Missing prompt-contract sections are flagged in strict doc lint runs.

**Tests (must exist before implementation)**

Unit:
- `test_doc_lint_rejects_missing_output_contract_section`
- `test_doc_lint_rejects_missing_instruction_priority_section`

Invariant:
- `test_doc_lint_accepts_prompt_contract_sections_for_spec_and_plan`

Integration:
- `test_check_fast_feedback_includes_updated_doc_lint_behavior`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `scripts/doc-lint.sh --path docs/specs/prompt-guidance-template-hardening.md --strict-new`
Expected: fails when required prompt-contract sections are absent from fixture/input.

**Implementation Steps**

1. Extend doc-lint required-section maps for spec and plan profiles.
2. Add targeted Bats assertions for new required sections.
3. Update docs task README with short operator note on the new lint requirement.

**Green Phase (required)**

Command: `scripts/check-fast-feedback.sh --changed`
Expected: updated lint checks pass in full fast-feedback flow.

**Refactor Phase (optional but controlled)**

Allowed scope: doc lint script and tests only
Re-run: `scripts/run-shell-tests.sh --changed`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 3: Add Model Compatibility Guidance and Example Contracts

**Files:**

- Modify: `docs/specs/prompt-guidance-template-hardening.md`
- Create: `docs/templates/examples/prompt-contract-minimal.md`
- Modify: `docs/tasks/README.md`

**Preconditions**

- Template and lint updates are complete.

**Invariants**

- Guidance clearly distinguishes stronger 5.4 adherence from 5.3 fallback behavior.
- Compatibility guidance remains conservative and tool-agnostic.

**Postconditions**

- Contributors can copy a minimal, valid prompt-contract example.
- Docs state explicit fallback behavior for GPT-5.3 when using XML-style delimiters.

**Tests (must exist before implementation)**

Unit:
- `test_prompt_contract_example_includes_priority_output_and_verification_blocks`

Invariant:
- `test_prompt_guidance_spec_documents_gpt53_plain_language_fallback`

Integration:
- `scripts/doc-lint.sh --changed --strict-new`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `test -f docs/templates/examples/prompt-contract-minimal.md`
Expected: fails before example file exists.

**Implementation Steps**

1. Add minimal example with short tagged blocks plus plain-language reinforcement.
2. Add compatibility section in the spec and task docs.
3. Re-run strict doc lint and fast-feedback.

**Green Phase (required)**

Command: `scripts/check-fast-feedback.sh --changed`
Expected: all checks pass with new docs/example.

**Refactor Phase (optional but controlled)**

Allowed scope: docs only
Re-run: `scripts/doc-lint.sh --changed --strict-new`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated
