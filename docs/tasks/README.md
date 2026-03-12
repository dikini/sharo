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
- Run dependency governance/security checks: `scripts/check-dependencies-security.sh`
- Run dependency governance/security checks for a commit range: `scripts/check-dependencies-security.sh --range <git-range>`
- Run Knot diff check: `scripts/check-knot-diff.sh --mapping docs/tasks/knot-diff-mapping.csv`
- Run research reference check: `scripts/check-research-references.sh --registry docs/tasks/research-reference-rules.csv`
- Run shell tests (Bats): `scripts/run-shell-tests.sh --all`
- Run shell tests (Bats) for a commit range: `scripts/run-shell-tests.sh --range <git-range>`
- Run shell formatting/lint checks: `scripts/check-shell-quality.sh --all`
- Run GitHub workflow lint checks: `scripts/check-workflows.sh`
- Run CI-smoke verification locally: `scripts/check-ci-smoke.sh`
- Run Rust hygiene checks (advisory): `scripts/check-rust-hygiene.sh --advisory --check all`
- Run Rust hygiene checks (strict): `scripts/check-rust-hygiene.sh --strict --check all`
- Run MVP matrix map quality gate: `scripts/check-mvp-matrix-map.sh`
- Run merge-result gate checks: `scripts/check-merge-result.sh`
- Run merge-compatibility gate alias: `scripts/check-merge-compat.sh`
- Run daemon invariant checks: `scripts/check-daemon-invariants.sh`
- Run deterministic conflict-policy checks: `scripts/check-conflict-determinism.sh`
- Run durability warning-signal checks: `scripts/check-durability-signals.sh`
- Run docs portability checks: `scripts/check-doc-portability.sh --changed`
- Run blocking local pre-push replay: `scripts/check-prepush-policy.sh`
- Run path-sensitive flaky daemon regression replay: `scripts/check-flaky-regressions.sh --changed`
- Canonical task runner entrypoint: `just verify`
- Canonical CI-smoke task runner entrypoint: `just verify-ci`
- Just wrapper for blocking local pre-push replay: `just prepush-policy`
- Just wrapper for docs portability checks: `just doc-portability`
- Just wrapper for path-sensitive flaky daemon regression replay: `just flaky-regressions`
- Initialize top-level starter files from templates: `scripts/init-repo.sh --apply`
- Just wrapper for starter template initialization: `just init-repo`
- Extract standalone reusable backbone directory: `scripts/extract-backbone.sh`
- Just wrapper for backbone extraction: `just extract-backbone`
- Initialize a new repository from extracted backbone: `scripts/init-from-backbone.sh --dest <path>`
- Just wrapper for backbone-based repo initialization: `just init-backbone-repo <dest> [project]`
- Bootstrap toolchain and workflow dependencies after fresh clone: `scripts/bootstrap-dev.sh --apply`
- Validate local bootstrap dependencies without installing: `scripts/bootstrap-dev.sh --check`
- Full fresh-clone readiness gate (run by bootstrap apply): `scripts/check-fast-feedback.sh --all`
- Strict docs lint now enforces prompt-contract headings for strict-profile specs/plans on changed files.
- Run protocol property tests: `cargo test -p sharo-core --test protocol_tests prop_protocol_roundtrip_preserves_task_summary_fields`
- Run daemon loom model checks: `cargo test -p sharo-daemon --test loom_submit_shutdown -- --nocapture`
- Run live OpenAI smoke manually (opt-in): `scripts/openai-live-smoke.sh`
- Enable live OpenAI smoke in fast-feedback (opt-in): `SHARO_ENABLE_LIVE_OPENAI_SMOKE=true scripts/check-fast-feedback.sh --changed`
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
- `TASK-DEV-BOOTSTRAP-001`: added deterministic fresh-clone bootstrap flow for required local tools and hooks.
- `TASK-REPO-INIT-001`: added deterministic repository starter-template initialization flow for top-level `README.md` and `AGENTS.md`.
- `TASK-BACKBONE-EXTRACT-001`: added deterministic backbone extraction flow producing `backbone/project-template` from reusable workflow/tooling/template assets.
- `TASK-BACKBONE-INIT-001`: added deterministic repo creation command from `backbone/project-template` with optional initial commit automation.
- `TASK-WORKFLOW-TOOL-GUIDES-SPEC-001`: defined shell/workflow/rust-hygiene guide contracts with CI and decision-support expectations.
- `TASK-WORKFLOW-TOOL-GUIDES-PLAN-001`: implemented shell/workflow/rust-hygiene wrappers, CI gates, and operator docs.
- `TASK-PROMPT-GUIDANCE-SPEC-001`: defined prompt-contract hardening for spec/plan templates and cross-model (GPT-5.3/GPT-5.4) compatibility guidance.
- `TASK-PROMPT-GUIDANCE-PLAN-001`: implemented prompt-contract sections, lint enforcement, and template usage guidance updates.
- `TASK-LOCAL-POLICY-REPLAY-SPEC-001`: defined blocking local pre-push replay, docs portability, and path-sensitive flaky daemon replay contracts.
- `TASK-LOCAL-POLICY-REPLAY-PLAN-001`: implemented blocking pre-push replay scripts, docs portability gating, flaky replay wiring, and operator docs.

## Tool Usage Guide

- Use `scripts/check-shell-quality.sh --all` before changing shell scripts to catch formatting/lint issues early.
- Use `scripts/check-workflows.sh` before changing `.github/workflows/*` to catch invalid actions syntax and expressions.
- Workflow lint is local-first: fast-feedback and pre-push enforce `scripts/check-workflows.sh`, while `policy-checks` no longer runs a dedicated CI `actionlint` step.
- Use `scripts/check-rust-hygiene.sh --advisory --check all` during feature work for dependency hygiene signal without blocking iteration.
- Use `scripts/check-rust-hygiene.sh --strict --check all` before dependency bumps or release preparation.
- Use `scripts/check-doc-portability.sh --changed` before committing docs changes to catch workstation-specific and worktree-local references early.
- Use `scripts/check-prepush-policy.sh` or `just prepush-policy` before manual pushes when you want the same blocking replay the `pre-push` hook will enforce.
- Use `scripts/check-flaky-regressions.sh --changed` when daemon/runtime paths changed and you want an explicit replay of the known unstable regression set before pushing.
- `policy-checks` runs dependency-security and shell tests in range-sensitive mode, so docs-only and unrelated Rust/runtime changes skip those heavier CI lanes.
- `policy-checks` now also avoids nightly toolchain and fuzz installation entirely; nightly fuzz/toolchain coverage lives in `.github/workflows/nightly-fuzz.yml`.
- `cargo semver-checks` is scoped to `sharo-core` because that crate is the public library surface in this workspace.
- Live OpenAI checks are opt-in only (cost/time sensitive); default hooks/fast-feedback remain non-network by default.
- Default smoke coverage still validates daemon CLI path via deterministic and fake-daemon scenarios in `scripts/tests/test-openai-live-smoke.bats`.

## Fresh Clone Tool Bootstrap

- Required local tools for full workflow checks:
  - system: `shellcheck`, `shfmt`, `actionlint`
  - cargo: `cargo-nextest`, `cargo-deny`, `cargo-audit`, `cargo-udeps`, `cargo-msrv`, `cargo-semver-checks`
- After clone:
  - run `scripts/bootstrap-dev.sh --check` to detect missing dependencies.
  - install missing system tools (`shellcheck`, `shfmt`) via package manager.
  - `scripts/bootstrap-dev.sh --apply` installs pinned `actionlint` into `.tools/actionlint` when missing.
  - bootstrap verifies `actionlint` archive integrity using:
    - pinned checksum-file SHA-256
    - release metadata asset digest match
    - archive SHA-256 match before extraction
    - installed binary version match against pinned release
  - bootstrap supports SHA-256 verification via `sha256sum` or `shasum -a 256`.
  - upstream `actionlint` releases currently do not publish detached archive signatures (`.sig`/`.asc`), so bootstrap enforces checksum/digest integrity verification instead of signature verification.
  - run `scripts/bootstrap-dev.sh --apply` to install project-managed tools/hooks and execute full verification.
  - local fast-feedback enforces workflow lint by default; CI `just verify` warns and skips only when `actionlint` is unavailable because workflow lint is intentionally local-only there.
  - per-push CI uses `just verify-ci` as a smoke gate and leaves nightly/fuzz-only work to the nightly workflow.

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
