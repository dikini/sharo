# AGENTS

## Project Language Constraint

- This project uses `Rust` as the implementation language.
- Rust edition is `2024`.
- Rust version **MUST** be `1.93` or higher.
- New runtime or core logic should be written in Rust unless explicitly documented otherwise.

## Changelog and Commits

- The project uses Common Changelog: <https://common-changelog.org/>.
- Task completion **MUST** update `CHANGELOG.md`.
- The project uses Conventional Commits v1.0.0: <https://www.conventionalcommits.org/en/v1.0.0/>.
- Commit messages **MUST** conform to the Conventional Commits specification.

## Canonical Source and Sync Policy

- The `git main` branch in this repository is the single source of truth.
- Any path outside this repository (including Knot vault paths) is non-canonical.
- Sync to or from non-repo paths **MUST** be explicitly requested for each task.
- Sync requests **MUST** be unambiguous and include:
  - exact source path
  - exact target path
  - direction of sync
  - scope of files/notes to sync
- No implicit mirroring between repo and non-repo paths is allowed.

## Staged Sync Protocol

- For explicit repo-vault sync tasks, use staged protocol artifacts in `docs/sync/`.
- Every sync operation should include:
  - one manifest file derived from `docs/sync/sync-manifest.template.json`
  - one evidence note derived from `docs/sync/sync-evidence.template.md`
- Validate sync manifests before completion:
  - `scripts/check-sync-manifest.sh --changed`
- Dry-run staged sync checks before external updates:
  - `scripts/sync-check.sh --dry-run --manifest <manifest>`

## Documentation Usage (`docs/`)

- `docs/specs/` stores canonical specifications (source of truth for behavior and invariants).
- `docs/plans/` stores execution plans used to implement or align specs.

Use `docs/specs/` first, then create/update a plan in `docs/plans/` before larger changes.

Example:

1. Update or read the MVP spec at `docs/specs/mvp.md`.
2. Create an implementation/alignment plan at `docs/plans/2026-03-04-<topic>-plan.md`.
3. Execute work against that plan and keep spec/plan references in sync.

## Templates and TDD Planning

- Reuse templates from `docs/templates/` when creating new docs:
  - `spec.template.md` for new specs
  - `plan.template.md` for new plans
  - `CHANGELOG.template.md` when initializing changelog structure
- Create new specs/plans via `scripts/doc-new.sh` (or `scripts/doc-start.sh`).
- For strict-profile docs, use `--strict-filled`:
  - `scripts/doc-new.sh spec <slug> --strict-filled`
  - `scripts/doc-new.sh plan <slug> --strict-filled`
  - `scripts/doc-start.sh` applies strict-filled scaffolding by default.
- Specs and plans should use `Template-Profile: tdd-strict-v1` unless explicitly waived.
- Under the strict profile, each task should include:
  - Preconditions
  - Invariants
  - Postconditions
  - Tests/checks defined before implementation (`Unit`, `Invariant`, `Integration`; `Property-based` optional only when using generative frameworks)
  - Red/Green phases and completion evidence
- At the start of any docs/spec/plan task, run `scripts/doc-lint.sh --changed --strict-new`.
- Run `scripts/doc-lint.sh --changed --strict-new` before committing documentation changes.
- Enable hooks once per clone with `scripts/install-hooks.sh`.

## Policy Enforcement (Machine Checks)

- Install local hooks once per clone: `scripts/install-hooks.sh`.
- Local `pre-commit` enforces:
  - `CHANGELOG.md` is staged for task-completion commits
  - Rust policy (`edition = 2024`, `rust-version >= 1.93`) when `Cargo.toml` exists
  - Rust workspace tests for Rust-impacting changes (`scripts/check-rust-tests.sh --changed`)
  - docs lint for changed files (`scripts/doc-lint.sh --changed --strict-new`)
  - docs terminology checks for changed files (`scripts/check-doc-terms.sh --changed`)
  - task registry validity (`scripts/check-tasks-registry.sh`)
  - task registry sync for changed specs/plans/scripts (`scripts/check-tasks-sync.sh --changed`)
- Local `commit-msg` enforces Conventional Commits.
- CI (`.github/workflows/policy-checks.yml`) re-checks docs lint, docs terminology, task registry validity/sync, Rust policy, Rust workspace tests, conventional commits, and changelog updates over push/PR commit range.

## Fast Feedback Loop (REQUIRED)

To catch problems at the earliest possible moment, contributors **MUST** run checks immediately after each relevant edit batch, not only at commit time.

1. Docs/spec/plan/task edits:
  - `scripts/check-fast-feedback.sh`

2. First Rust-impacting edit in a batch:
  - `scripts/check-fast-feedback.sh`

3. Every subsequent Rust edit batch:
  - `scripts/check-fast-feedback.sh`

4. Before commit:
  - contributors **MUST** have a fresh fast-feedback marker from `scripts/check-fast-feedback.sh`.
  - pre-commit rejects commits when `scripts/check-fast-feedback.sh` was not run on the current tree state.

Skipping this loop is a policy violation because it delays failure detection and increases rework.
