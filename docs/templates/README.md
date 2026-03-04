# Templates

These templates standardize project artifacts while keeping dependencies minimal.

## Available templates

- `CHANGELOG.template.md`
  - Common Changelog baseline.
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
- Start docs task with:
  - `scripts/doc-start.sh spec <slug>`
  - `scripts/doc-start.sh plan <slug>`
- Run lint at task start and before commit:
  - `scripts/doc-lint.sh --changed --strict-new`
