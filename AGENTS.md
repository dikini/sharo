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

## Documentation Usage (`docs/`)

- `docs/specs/` stores canonical specifications (source of truth for behavior and invariants).
- `docs/plans/` stores execution plans used to implement or align specs.

Use `docs/specs/` first, then create/update a plan in `docs/plans/` before larger changes.

Example:

1. Update or read the MVP spec at `docs/specs/mvp.md`.
2. Create an implementation/alignment plan at `docs/plans/2026-03-04-<topic>-plan.md`.
3. Execute work against that plan and keep spec/plan references in sync.
