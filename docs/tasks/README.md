# Task Registry

This directory provides deterministic task state listing for planning and deferred work.

## Format

- Registry file: `docs/tasks/tasks.csv`
- Header columns:
  - `id,type,title,source,status,blocked_by,notes`
- Status enum:
  - `planned`
  - `deferred`
  - `in_progress`
  - `done`
  - `cancelled`

## Source Reference Rule

Each row must satisfy all of the following:

- `source` path exists in the repository.
- `id` appears in the `source` file content.

This keeps task state deterministic and reduces stale registry entries.

## Commands

- List all: `scripts/tasks.sh`
- List planned: `scripts/tasks.sh --status planned`
- List deferred: `scripts/tasks.sh --status deferred`
- Summary: `scripts/tasks.sh --summary`
- Validate registry: `scripts/check-tasks-registry.sh`
- Validate sync gating (changed files): `scripts/check-tasks-sync.sh --changed`

## Known Deferred Items

- `TASK-KNOT-DIFF-001`: live repo↔Knot content diff checker, blocked by Knot CLI API stability.
- `TASK-RESEARCH-LINT-001`: research citation/reference verifier automation, blocked by Knot-side workflow readiness.
- `TASK-IPC-TRANSPORT-001`: replace stub client path with real CLI↔daemon IPC transport after vertical-slice stabilization.

## Completed Bootstrap

- `TASK-TASKS-REGISTRY-001`: deterministic task registry and machine checks are implemented.
- `TASK-RUST-WORKSPACE-001`: Rust workspace bootstrap with `sharo-core`, `sharo-cli`, and `sharo-daemon` is implemented.
- `TASK-VERIFICATION-GATE-001`: Rust-impacting commits are blocked unless `cargo test --workspace` passes.
- `TASK-FAST-FEEDBACK-001`: single-command fast-feedback checks and freshness marker enforcement are implemented.
