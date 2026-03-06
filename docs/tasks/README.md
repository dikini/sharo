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
- List in-progress: `scripts/tasks.sh --status in_progress`
- List deferred: `scripts/tasks.sh --status deferred`
- List done: `scripts/tasks.sh --status done`
- Summary: `scripts/tasks.sh --summary`
- Upsert task row (create/update): `scripts/tasks.sh --upsert <id> --status <status> [--type ... --title ... --source ... --blocked-by ... --notes ...]`
- Validate registry: `scripts/check-tasks-registry.sh`
- Validate sync gating (changed files): `scripts/check-tasks-sync.sh --changed`
- Run Rust workspace tests with `nextest` fallback: `scripts/check-tests.sh --workspace`
- Run Knot diff check: `scripts/check-knot-diff.sh --mapping docs/tasks/knot-diff-mapping.csv`
- Run research reference check: `scripts/check-research-references.sh --registry docs/tasks/research-reference-rules.csv`
- Run shell tests (Bats): `scripts/run-shell-tests.sh --all`
- Run MVP matrix map quality gate: `scripts/check-mvp-matrix-map.sh`
- Run merge-result gate checks: `scripts/check-merge-result.sh`
- MVP matrix mapping file: `docs/tasks/mvp-verification-matrix-map.csv`

## MVP Slice Tracking

- `TASK-MVP-SLICE-000`: MVP roadmap and tracking bootstrap.
- `TASK-MVP-SLICE-001`: Scenario A read-oriented end-to-end path.
- `TASK-MVP-SLICE-002`: Scenario B policy and approval-gated restricted path.
- `TASK-MVP-SLICE-003`: Scenario C overlap and coordination durability.
- `TASK-MVP-SLICE-004`: MVP protocol and CLI surface completion.
- `TASK-MVP-SLICE-005`: verification matrix closure and hardening.
- `TASK-DOC-TEMPLATE-TERMS-001`: strict template terminology alignment for invariant vs property-based tests.
- `TASK-FAST-FEEDBACK-ERGONOMICS-001`: content-based marker validity and pre-commit auto-refresh for stale markers.
- `TASK-DOC-STRICT-FILLED-001`: strict-filled scaffolding for spec/plan docs and lint guidance hints.
- `TASK-TASKS-UPSERT-001`: task-registry upsert helper for ergonomic create/update operations.

Current state: slices 000 through 005 are marked `done` in `docs/tasks/tasks.csv`.

## Completed Tooling Items

- `TASK-KNOT-DIFF-001`: implemented read-only repo↔Knot mapping diff checker using normalized content hashes.
- `TASK-RESEARCH-LINT-001`: implemented registry-driven research citation/addendum verifier with marker and path checks.
- `TASK-BATS-TESTS-001`: migrated shell-script test harnesses to `bats-core` with deterministic installer and unified runner.

## Tooling Inputs

- Knot diff mapping: `docs/tasks/knot-diff-mapping.csv`
  - Header: `canonical_path,knot_path`
  - `canonical_path` must resolve to a repository file.
  - `knot_path` is passed to `knot tool get_note`.
- Research reference registry: `docs/tasks/research-reference-rules.csv`
  - Header: `note_path,required_markers,required_refs`
  - `required_markers` uses `;` as a separator.
  - `required_refs` uses `;` as a separator.
  - Each listed reference path must both exist and appear verbatim in the note content.

## Completed Bootstrap

- `TASK-TASKS-REGISTRY-001`: deterministic task registry and machine checks are implemented.
- `TASK-RUST-WORKSPACE-001`: Rust workspace bootstrap with `sharo-core`, `sharo-cli`, and `sharo-daemon` is implemented.
- `TASK-VERIFICATION-GATE-001`: Rust-impacting commits are blocked unless `cargo test --workspace` passes.
- `TASK-FAST-FEEDBACK-001`: single-command fast-feedback checks and freshness marker enforcement are implemented.
- `TASK-IPC-TRANSPORT-001`: real CLI↔daemon IPC transport is implemented with Unix socket JSON request/response flow.
