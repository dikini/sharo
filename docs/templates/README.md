# Templates

These templates standardize project artifacts while keeping dependencies minimal.

## Available templates

- `CHANGELOG.template.md`
  - Common Changelog baseline.
- `README.template.md`
  - Top-level project starter with status, layout, quick start, workflow, verification, and contribution sections.
- `AGENTS.template.md`
  - Project governance starter covering language/tooling policy, docs workflow, checks, and fast-feedback expectations.
- `spec.template.md`
  - Spec structure with task contracts and verification focus.
- `plan.template.md`
  - Strict TDD execution plan format.

## Strict TDD profile

`spec.template.md` and `plan.template.md` use:

- `Template-Profile: tdd-strict-v1`

When a document includes that marker, `scripts/doc-lint.sh` enforces required sections and ordering constraints.

This allows gradual adoption:

- New specs/plans should use the strict template profile.
- Older documents remain valid until explicitly migrated.

## Deterministic usage

- Create new docs with:
  - `scripts/doc-new.sh spec <slug>`
  - `scripts/doc-new.sh plan <slug>`
- Recommended for strict-profile docs:
  - `scripts/doc-new.sh spec <slug> --strict-filled`
  - `scripts/doc-new.sh plan <slug> --strict-filled`
- Start docs task with:
  - `scripts/doc-start.sh spec <slug>`
  - `scripts/doc-start.sh plan <slug>`
  - `doc-start` applies strict-filled scaffolding by default.
- Initialize top-level starter files from templates:
  - `scripts/init-repo.sh --apply`
  - optional overwrite mode: `scripts/init-repo.sh --apply --force`
- Extract a standalone reusable backbone package:
  - `scripts/extract-backbone.sh`
  - optional target path: `scripts/extract-backbone.sh --target <path>`
- Initialize a new repository from extracted backbone:
  - `scripts/init-from-backbone.sh --dest <path>`
  - optional project override: `scripts/init-from-backbone.sh --dest <path> --project <name>`
- Run lint at task start and before commit:
  - `scripts/doc-lint.sh --changed --strict-new`

## Prompt Contract Sections

Strict-profile spec/plan templates now include deterministic prompt-contract sections:

- `## Instruction Priority`
- `## Output Contract`
- `## Evidence / Verification Contract` (spec template)
- `## Execution Mode` (plan template)
- `## Task Update Contract` (plan template)
- `## Completion Gate` (plan template)
- `## Model Compatibility Notes`

## Delimiter Block Guidance

- XML-style blocks such as `<context>`, `<constraints>`, or `<output_contract>` may be used as readability aids in prompts and examples.
- These tags are plain-text conventions, not parser requirements.
- Critical constraints must always be duplicated in plain language so behavior remains stable across model variants.
- Minimal example: `docs/templates/examples/prompt-contract-minimal.md`
