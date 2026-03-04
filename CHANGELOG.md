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
- Added machine-enforced policy checks:
  - `scripts/check-conventional-commit.sh`
  - `scripts/check-changelog-staged.sh`
  - `scripts/check-rust-policy.sh`
  - `scripts/check-doc-terms.sh`
- Added local `commit-msg` hook and extended local `pre-commit` hook to enforce:
  - conventional commit messages
  - changelog staging
  - Rust policy
  - docs lint for changed files
- Added CI workflow `.github/workflows/policy-checks.yml` to enforce policy checks on `push` and `pull_request`.
- Added sync protocol docs for staged repo-vault operations without direct vault CLI:
  - `docs/specs/vault-sync-protocol.md`
  - `docs/plans/2026-03-04-vault-sync-protocol-plan.md`
- Added Task 1 sync artifacts for staged protocol execution:
  - `docs/sync/README.md`
  - `docs/sync/sync-manifest.template.json`
  - `docs/sync/sync-evidence.template.md`
- Updated `docs/specs/vault-sync-protocol.md` references to include sync artifact templates.
- Added sync manifest validator and staged sync runner:
  - `scripts/check-sync-manifest.sh`
  - `scripts/sync-check.sh`
  - `docs/sync/examples/valid.manifest.json`
- Added sync tool verification fixtures and test harness:
  - `scripts/tests/sync/invalid.missing-sync-id.manifest.json`
  - `scripts/tests/test-sync-tools.sh`
- Added canonical alias tooling for repo artifacts:
  - `docs/aliases.toml`
  - `scripts/alias-resolve.sh`
- Added alias design and implementation planning docs:
  - `docs/plans/2026-03-04-alias-resolution-design.md`
  - `docs/plans/2026-03-04-alias-resolution-implementation-plan.md`

### Changed

- Improved Rust policy version comparison logic in `scripts/check-rust-policy.sh` to use semantic version component comparison.
- Updated `scripts/install-hooks.sh` to mark policy check scripts executable during hook installation.
- Updated `AGENTS.md` with a dedicated machine-enforcement section (local hooks + CI expectations).
- Extended local and CI policy enforcement with sync manifest checks.
- Extended local and CI policy enforcement with documentation terminology checks.
- Updated `AGENTS.md` with staged sync protocol guidance and required commands.
- Updated `docs/specs/mvp.md` assumptions to align with canonical repo policy and explicit sync protocol.
- Marked completion evidence checklists in `docs/plans/2026-03-04-vault-sync-protocol-plan.md` as done.
- Resolved `docs/specs/vault-sync-protocol.md` open questions with explicit MVP decisions:
  - all-or-nothing promotion policy
  - manual-only push-back retries
  - shell-level manifest checks as MVP schema authority
- Updated strict spec formatting to reduce ambiguity between normative requirements and task tracking:
  - converted `docs/specs/vault-sync-protocol.md` requirement checklists to plain bullets
  - updated `docs/templates/spec.template.md` to use plain bullets for Preconditions/Invariants/Postconditions/Tests
