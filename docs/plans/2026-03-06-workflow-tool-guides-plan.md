# Workflow Tool Guides Implementation Plan

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Invariant, and Integration checks.
> - Use Property-based tests only when a generative framework is actually used.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: add deterministic operator guides and machine-check wrappers for shell quality, workflow linting, and rust hygiene tooling.
Architecture: deliver in three bounded tasks. First add wrappers/tests and local task-runner targets. Then wire CI/bootstrap guardrails. Finally document usage and policy changes, including LGPL-compatible license allowlist updates. Keep heavy rust hygiene checks in scheduled/manual CI to avoid per-PR latency spikes.
Tech Stack: Bash scripts, Bats, GitHub Actions, just, cargo-udeps, cargo-msrv, cargo-semver-checks, actionlint, shellcheck, shfmt.
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-WORKFLOW-TOOL-GUIDES-SPEC-001, TASK-WORKFLOW-TOOL-GUIDES-PLAN-001

---

### Task 1: Add wrappers and local entry points

**Files:**

- Create: `scripts/check-shell-quality.sh`
- Create: `scripts/check-workflows.sh`
- Create: `scripts/check-rust-hygiene.sh`
- Create: `scripts/tests/test-check-shell-quality.bats`
- Create: `scripts/tests/test-check-workflows.bats`
- Create: `scripts/tests/test-check-rust-hygiene.bats`
- Modify: `justfile`
- Modify: `scripts/tests/test-justfile-targets.bats`

**Preconditions**

- Existing shell test harness is green.

**Invariants**

- Wrapper scripts expose deterministic argument parsing and failure semantics.
- `just` targets delegate directly to canonical scripts.

**Postconditions**

- Contributors can run `just shell-quality`, `just workflow-lint`, and `just rust-hygiene`.

**Tests (must exist before implementation)**

Unit:
- `test_check_shell_quality_enforces_shellcheck_and_shfmt_presence`
- `test_check_workflows_requires_actionlint`
- `test_check_rust_hygiene_supports_strict_and_advisory_modes`

Invariant:
- `test_justfile_includes_required_workflow_targets`

Integration:
- `test_justfile_wires_rust_hygiene_command`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `bats scripts/tests/test-check-shell-quality.bats scripts/tests/test-check-workflows.bats scripts/tests/test-check-rust-hygiene.bats`
Expected: failing because wrapper tests/scripts are not yet present.

**Implementation Steps**

1. Add wrapper scripts with deterministic usage/help and explicit missing-tool guidance.
2. Add Bats checks for script contracts.
3. Add `just` targets and expand `test-justfile-targets.bats`.

**Green Phase (required)**

Command: `bats scripts/tests/test-check-shell-quality.bats scripts/tests/test-check-workflows.bats scripts/tests/test-check-rust-hygiene.bats scripts/tests/test-justfile-targets.bats`
Expected: all task tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `scripts/check-*.sh`, `scripts/tests/test-check-*.bats`, `justfile`
Re-run: `bats scripts/tests/test-check-shell-quality.bats scripts/tests/test-check-workflows.bats scripts/tests/test-check-rust-hygiene.bats scripts/tests/test-justfile-targets.bats`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 2: Wire CI and bootstrap guardrails

**Files:**

- Modify: `.github/workflows/policy-checks.yml`
- Create: `.github/workflows/rust-hygiene.yml`
- Modify: `scripts/bootstrap-dev.sh`
- Modify: `scripts/tests/test-bootstrap-dev.bats`

**Preconditions**

- Wrapper scripts are present and test-covered.

**Invariants**

- Required PR checks stay lightweight and deterministic.
- Heavy rust hygiene checks are off the critical PR path.

**Postconditions**

- Policy checks run workflow/shell wrappers.
- Scheduled/manual workflow runs strict rust hygiene checks.
- Bootstrap checks include workflow/shell/rust-hygiene dependencies.

**Tests (must exist before implementation)**

Unit:
- `test_bootstrap_checks_shell_and_workflow_lint_prerequisites`

Invariant:
- `test_policy_checks_workflow_invokes_workflow_and_shell_wrappers`

Integration:
- `test_rust_hygiene_workflow_runs_strict_hygiene_script`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `rg 'scripts/check-workflows\\.sh|scripts/check-shell-quality\\.sh --all' .github/workflows/policy-checks.yml && rg 'ensure_system_tool actionlint' scripts/bootstrap-dev.sh`
Expected: one or more checks are missing before implementation.

**Implementation Steps**

1. Install required tooling in policy-checks and run workflow/shell wrappers.
2. Add scheduled/manual rust hygiene workflow for strict checks.
3. Expand bootstrap checks to include system + cargo hygiene dependencies.

**Green Phase (required)**

Command: `bats scripts/tests/test-bootstrap-dev.bats scripts/tests/test-check-workflows.bats scripts/tests/test-check-shell-quality.bats`
Expected: all task tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `.github/workflows/*.yml`, `scripts/bootstrap-dev.sh`, related Bats tests
Re-run: `scripts/check-workflows.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated

### Task 3: Document usage and policy updates

**Files:**

- Modify: `docs/tasks/README.md`
- Modify: `deny.toml`
- Modify: `docs/tasks/tasks.csv`
- Modify: `CHANGELOG.md`
- Create: `docs/specs/workflow-tool-guides.md`
- Create: `docs/plans/2026-03-06-workflow-tool-guides-plan.md`

**Preconditions**

- Implementation and CI/bootstrap wiring are complete.

**Invariants**

- Task registry and source-reference constraints remain valid.
- docs lint strict profile remains green.

**Postconditions**

- Operator-facing guide includes when/how to run each tool and strict/advisory guidance.
- License allowlist records compatible LGPL variants explicitly.
- changelog and task registry capture rollout evidence.

**Tests (must exist before implementation)**

Unit:
- `test_docs_readme_lists_shell_workflow_rust_hygiene_commands`

Invariant:
- `scripts/doc-lint.sh --changed --strict-new`
- `scripts/check-tasks-registry.sh`

Integration:
- `scripts/check-fast-feedback.sh --all`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: no new files in scope before docs edits.

**Implementation Steps**

1. Update docs command reference and tool-usage guidance in `docs/tasks/README.md`.
2. Add compatible LGPL variants in `deny.toml`.
3. Update task registry rows and changelog entries for this rollout.

**Green Phase (required)**

Command: `scripts/check-fast-feedback.sh --all`
Expected: all checks pass with fresh marker and updated docs/task registry.

**Refactor Phase (optional but controlled)**

Allowed scope: docs and policy config touched by this plan
Re-run: `scripts/doc-lint.sh --changed --strict-new`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG.md updated
