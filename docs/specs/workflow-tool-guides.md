# Workflow Tool Guides

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Define tests before implementation work.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Invariant, and Integration checks.
> - Use Property-based tests only when a generative framework is actually used.
> - Run `scripts/doc-lint.sh` before commit.

Updated: 2026-03-06
Status: active
Owner: platform
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-WORKFLOW-TOOL-GUIDES-SPEC-001, TASK-WORKFLOW-TOOL-GUIDES-PLAN-001

## Purpose

Define deterministic operator guidance and machine-check surfaces for shell quality, GitHub workflow linting, and Rust hygiene checks (`udeps`, `msrv`, `semver`) so fresh clones and CI can enforce consistent workflow standards.

## Scope

### In Scope

- Add canonical script wrappers for shell formatting/lint, workflow lint, and rust hygiene checks.
- Add guidance in task docs and bootstrap to make prerequisite tooling explicit for fresh clones.
- Add required CI checks for shell/workflow quality and scheduled strict rust hygiene checks.
- Expand dependency policy allowlist to include compatible LGPL variants.

### Out of Scope

- Runtime behavior changes for daemon/core logic.
- Cross-platform package-manager automation for system dependencies.
- Turning rust hygiene checks into required per-PR gates (kept scheduled/manual to preserve PR latency).

## Core Terms

- `Shell Quality Gate`: deterministic `shfmt` and `shellcheck` verification over scoped shell files.
- `Workflow Lint Gate`: deterministic `actionlint` verification for GitHub workflow files.
- `Rust Hygiene`: maintenance checks for unused dependencies, minimum supported Rust version viability, and public API semver compatibility.
- `Advisory Mode`: non-blocking check execution that warns on failures or missing optional tools.
- `Strict Mode`: blocking check execution that fails on missing tools or check failures.

## Interfaces / Contracts

- `scripts/check-shell-quality.sh --all|--changed` is the canonical shell lint/format wrapper.
- `scripts/check-workflows.sh` is the canonical GitHub workflow lint wrapper.
- `scripts/check-rust-hygiene.sh --advisory|--strict --check all|udeps|msrv|semver` is the canonical rust hygiene wrapper.
- `scripts/bootstrap-dev.sh --check|--apply` must detect missing tools for these wrappers and provide deterministic install hints.
- `just` command map must expose stable entry points for the new wrappers:
  - `just shell-quality`
  - `just workflow-lint`
  - `just rust-hygiene`

## Invariants

- Local and CI checks use the same wrapper scripts rather than duplicating command logic.
- Missing-tool behavior is explicit, actionable, and deterministic.
- Rust hygiene semver checks only target `sharo-core` (public library crate) unless spec is explicitly revised.
- CI required checks remain bounded in runtime; heavy rust hygiene checks are moved to scheduled/manual workflow.

## Task Contracts

### Task 1: Add shell/workflow/rust-hygiene wrappers and local entry points

**Preconditions**

- Existing scripts and Bats harness are passing.

**Invariants**

- Wrapper scripts provide one command per check surface with deterministic argument parsing.
- `just` targets remain thin delegators.

**Postconditions**

- Contributors can run deterministic wrappers locally for shell, workflow, and rust hygiene checks.

**Tests (must exist before implementation)**

Unit:
- `test_check_shell_quality_enforces_shellcheck_and_shfmt_presence`
- `test_check_workflows_requires_actionlint`
- `test_check_rust_hygiene_supports_strict_and_advisory_modes`

Invariant:
- `test_justfile_includes_shell_workflow_and_rust_hygiene_targets`

Integration:
- `test_policy_checks_workflow_invokes_workflow_and_shell_wrappers`

Property-based (optional):
- not applicable

### Task 2: Add CI and bootstrap guardrails for prerequisite tooling

**Preconditions**

- Wrapper scripts exist and pass script-level tests.

**Invariants**

- Required PR CI checks remain deterministic and reproducible locally.
- Heavy rust hygiene checks execute outside main PR gate path.

**Postconditions**

- `policy-checks` enforces workflow/shell wrappers.
- `rust-hygiene` workflow runs strict periodic checks.
- bootstrap surfaces required tool dependencies for fresh clones.

**Tests (must exist before implementation)**

Unit:
- `test_bootstrap_checks_shell_and_workflow_lint_prerequisites`
- `test_rust_hygiene_script_includes_udeps_msrv_and_semver_checks`

Invariant:
- `test_policy_checks_workflow_invokes_workflow_and_shell_wrappers`

Integration:
- `test_rust_hygiene_workflow_runs_strict_hygiene_script`

Property-based (optional):
- not applicable

### Task 3: Document usage and dependency selection policy refinements

**Preconditions**

- Tool wrappers and CI wiring are implemented.

**Invariants**

- Docs remain the canonical operator-facing command reference.
- License allowlist change remains explicit and auditable.

**Postconditions**

- `docs/tasks/README.md` includes practical usage guidance for each wrapper.
- `deny.toml` includes compatible LGPL variants.
- changelog records the rollout.

**Tests (must exist before implementation)**

Unit:
- `test_docs_readme_lists_shell_workflow_rust_hygiene_commands`

Invariant:
- `scripts/doc-lint.sh --changed --strict-new`

Integration:
- `scripts/check-fast-feedback.sh --all`

Property-based (optional):
- not applicable

## Scenarios

- S1: Contributor edits a shell script and catches formatting/lint regressions before commit.
- S2: Contributor edits workflow YAML and catches expression/schema issues via `actionlint` before push.
- S3: Weekly rust hygiene run detects semver/public API drift relative to `origin/main`.
- S4: Fresh clone user runs bootstrap and gets deterministic missing-tool guidance.

## Verification

- `scripts/doc-lint.sh --changed --strict-new`
- `scripts/check-fast-feedback.sh --all`
- `scripts/check-tasks-registry.sh`
- `scripts/check-tasks-sync.sh --changed`

## Risks and Failure Modes

- `cargo msrv` and `cargo semver-checks` can be runtime-heavy; run them on schedule/manual to avoid blocking normal PR cadence.
- Some environments cannot auto-install system tools without elevated privileges; bootstrap must provide explicit fallback commands.
- `cargo +nightly udeps` can fail when nightly is unavailable; strict mode should fail loudly and advisory mode should warn.

## Open Questions

- Should strict rust hygiene be promoted from scheduled/manual to required PR status after burn-in and runtime measurement?

## References

- [workflow-tooling-rollout.md](/home/dikini/Projects/sharo/.worktrees/workflow-tool-guides/docs/specs/workflow-tooling-rollout.md)
- [README.md](/home/dikini/Projects/sharo/.worktrees/workflow-tool-guides/docs/tasks/README.md)
