# Changelog

All notable changes to this project will be documented in this file.

The format is based on Common Changelog:
<https://common-changelog.org/>

## Unreleased

### Added

- Initialized Git repository.
- Added project governance in `AGENTS.md`:
  - Rust language, edition 2024, and minimum Rust version 1.93.
  - Common Changelog and Conventional Commits requirements.
  - Documentation workflow for `docs/specs` and `docs/plans`.
- Added MVP specification at `docs/specs/mvp.md`.
- Added planning docs:
  - `docs/plans/2026-03-04-design-note-alignment-plan.md`
  - `docs/plans/2026-03-04-research-note-alignment-plan.md`
  - `docs/plans/2026-03-04-doc-lint-gate-implementation-plan.md`
- Updated project governance in `AGENTS.md`:
  - canonical source is `git main`
  - non-repo sync requires explicit, unambiguous request (source, target, direction, scope)
- Added lightweight docs quality gate script:
  - `scripts/doc-lint.sh`
  - includes evergreen checks and temporary regression guard policy metadata
- Added deterministic docs scaffolding and start workflow:
  - `scripts/doc-new.sh`
  - `scripts/doc-start.sh`
- Added templates for core artifacts in `docs/templates/`:
  - `CHANGELOG.template.md`
  - `spec.template.md`
  - `plan.template.md`
  - `README.md`
- Extended docs linting:
  - supports `--changed`, `--path`, and `--strict-new` modes
  - enforces strict-template structure for `Template-Profile: tdd-strict-v1`
  - enforces strict profile marker on new specs/plans when `--strict-new` is used
- Added hook tooling:
  - `.githooks/pre-commit` runs docs lint for changed files
  - `scripts/install-hooks.sh` sets `core.hooksPath` to `.githooks`
- Updated `AGENTS.md` with deterministic template and lint workflow guidance.
