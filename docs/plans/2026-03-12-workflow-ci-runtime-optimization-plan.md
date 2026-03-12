# Workflow CI Runtime Optimization Implementation Plan

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Invariant, and Integration checks.
> - Use Property-based tests only when a generative framework is actually used.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: cut additional time from `policy-checks` by gating heavy installs before installation, by removing duplicated Rust verification, and by moving nightly/fuzz-only work into a dedicated nightly workflow.
Architecture: keep `policy-checks` as the fast merge-blocking lane, introduce one nightly verification workflow for nightly toolchain and fuzz coverage, and keep canonical behavior in repo scripts rather than embedding fragile logic directly into workflow YAML. Preserve `sccache` but tune CI execution to avoid unnecessary incremental/non-cacheable work and redundant test targets.
Tech Stack: Bash scripts, GitHub Actions, `sccache`, cargo nextest, cargo-deny, cargo-audit, cargo-fuzz, nightly Rust toolchain, Bats, just.
Template-Profile: tdd-strict-v1
Updated: 2026-03-12
Status: completed

Task-Registry-Refs: TASK-WORKFLOW-CI-RUNTIME-SPEC-001, TASK-WORKFLOW-CI-RUNTIME-PLAN-001

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level workflow and repo-governance policies.
3. This plan's execution and verification contracts.
4. Explicit plan updates recorded in this document.

## Execution Mode

- Mode: execute-with-checkpoints
- Default: execute-with-checkpoints unless user explicitly requests plan-only output.

## Output Contract

- Gate dependency-security and fuzz/nightly installs before tool installation in CI.
- Move nightly toolchain and fuzz work into a dedicated nightly workflow.
- Remove duplicated property/loom Rust coverage from `policy-checks`.
- Capture cache-oriented CI changes that improve `sccache` effectiveness without changing runtime behavior.

## Task Update Contract

- Any removed per-push step must name the replacement workflow or remaining coverage surface.
- Any CI install step added or retained must be justified by a pre-install scope decision.
- Rust test coverage changes must preserve named scenario/property/loom contracts.
- Timing comparisons must cite run `23010870993` as the baseline unless a newer equivalent run supersedes it.
- Post-rollout timing deltas remain pending until a fresh `policy-checks` run exists after implementation.

## Completion Gate

- `policy-checks` does not install dependency-security tools for ranges with no Cargo input changes.
- `policy-checks` does not install nightly or `cargo-fuzz` on runs that do not execute fuzz/nightly work.
- Nightly/fuzz verification exists in a dedicated scheduled workflow.
- Property and loom targets are executed only once per applicable workflow path.
- CI cache settings and resulting behavior are documented and verified.

## Model Compatibility Notes

- This rollout changes workflow/tooling behavior only.
- No Rust runtime, daemon protocol, or product-facing feature behavior changes are in scope.

### Task 1: Add pre-install gating for dependency-security and fuzz/nightly lanes

**Files:**

- Modify: `.github/workflows/policy-checks.yml`
- Modify: `scripts/tests/test-check-dependencies-security.bats`
- Modify: `scripts/tests/test-fuzz-gating.bats`
- Modify or create: CI-path classification shell/Bats coverage if needed

**Preconditions**

- Range resolution already exists early in `policy-checks`.
- `scripts/check-dependencies-security.sh --range` and `scripts/check-fuzz.sh` already support skip semantics after entry.

**Invariants**

- Cargo-impacting ranges still trigger dependency-security installation and execution.
- Fuzz/nightly installation occurs only in workflows that actually run fuzz/nightly verification.

**Postconditions**

- `policy-checks` decides whether the dependency-security and fuzz/nightly lanes are needed before installation steps.

**Tests (must exist before implementation)**

Unit:
- `policy_checks_installs_dependency_tools_only_for_cargo_ranges`
- `policy_checks_does_not_install_fuzz_tooling_in_per_push_lane`

Invariant:
- `resolve_commit_range_precedes_heavy_install_steps`

Integration:
- `docs_only_policy_checks_skip_dependency_and_fuzz_installs`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `scripts/install-bats.sh >/dev/null && "$(scripts/install-bats.sh)" scripts/tests/test-check-dependencies-security.bats scripts/tests/test-fuzz-gating.bats scripts/tests/test-justfile-targets.bats`
Expected: new install-gating expectations fail before workflow changes.

**Implementation Steps**

1. Add deterministic path classification for Cargo-impacting and fuzz-impacting ranges.
2. Gate dependency-security install before `cargo install --locked cargo-deny cargo-audit`.
3. Remove nightly and `cargo-fuzz` install from per-push `policy-checks`.
4. Keep CI logs explicit when lanes are skipped and why.

**Green Phase (required)**

Command: `scripts/install-bats.sh >/dev/null && "$(scripts/install-bats.sh)" scripts/tests/test-check-dependencies-security.bats scripts/tests/test-fuzz-gating.bats scripts/tests/test-justfile-targets.bats`
Expected: targeted CI install-gating shell coverage passes.

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Targeted shell tests passing

### Task 2: Add nightly workflow for nightly/fuzz verification

**Files:**

- Create: `.github/workflows/nightly-fuzz.yml`
- Modify: `scripts/tests/test-fuzz-gating.bats`
- Optionally modify: docs that enumerate workflow responsibilities

**Preconditions**

- The team accepts moving fuzz/nightly checks out of per-push CI.

**Invariants**

- Nightly workflow is automated and visible.
- Canonical fuzz behavior still lives in `scripts/check-fuzz.sh`.

**Postconditions**

- A dedicated nightly workflow installs nightly Rust and `cargo-fuzz`, then runs the designated fuzz verification.

**Tests (must exist before implementation)**

Unit:
- `nightly_fuzz_workflow_exists`
- `nightly_fuzz_workflow_installs_nightly_and_cargo_fuzz`

Invariant:
- `policy_checks_and_nightly_workflow_do_not_both_own_fuzz_installation`

Integration:
- `nightly_workflow_runs_check_fuzz_with_explicit_mode`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `scripts/install-bats.sh >/dev/null && "$(scripts/install-bats.sh)" scripts/tests/test-fuzz-gating.bats`
Expected: nightly-workflow expectations fail before the new workflow exists.

**Implementation Steps**

1. Create the nightly workflow with schedule and optional manual trigger.
2. Install nightly toolchain and `cargo-fuzz` only there.
3. Run the chosen fuzz mode (`--smoke` or `--full`) through `scripts/check-fuzz.sh`.
4. Update Bats coverage to reflect the split between per-push and nightly workflows.

**Green Phase (required)**

Command: `scripts/install-bats.sh >/dev/null && "$(scripts/install-bats.sh)" scripts/tests/test-fuzz-gating.bats`
Expected: nightly workflow shell coverage passes.

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Nightly workflow tests passing

### Task 3: Remove duplicated Rust coverage and tune CI compile behavior

**Files:**

- Modify: `.github/workflows/policy-checks.yml`
- Modify: `justfile` if adding `verify-ci`
- Modify: `scripts/check-fast-feedback.sh` or related CI smoke entrypoints if needed
- Modify: `scripts/tests/test-justfile-targets.bats`
- Modify or create: shell coverage for property/loom ownership and CI env settings

**Preconditions**

- Job logs show property and loom targets are already covered inside workspace tests.
- `sccache` stats show meaningful Rust cache reuse, but also `incremental` non-cacheable calls.

**Invariants**

- Property and loom checks remain covered at least once.
- CI uses deterministic compile settings compatible with `sccache`.

**Postconditions**

- `policy-checks` no longer executes duplicated property/loom Rust targets.
- CI sets `CARGO_INCREMENTAL=0` when relying on `sccache`.
- If added, `just verify-ci` avoids duplicating later full Rust workspace tests.

**Tests (must exist before implementation)**

Unit:
- `policy_checks_runs_property_target_once`
- `policy_checks_runs_loom_target_once`
- `policy_checks_sets_cargo_incremental_zero`

Invariant:
- `verify_ci_and_workspace_tests_have_no_unjustified_overlap`

Integration:
- `policy_checks_runtime_drops_after_duplicate_rust_coverage_removal`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `scripts/install-bats.sh >/dev/null && "$(scripts/install-bats.sh)" scripts/tests/test-fuzz-gating.bats scripts/tests/test-justfile-targets.bats`
Expected: duplicate-coverage and CI-env expectations fail before workflow updates.

**Implementation Steps**

1. Choose one owner for property/loom coverage in `policy-checks`.
2. Remove the duplicate steps or exclude those targets from the broader Rust lane.
3. Set `CARGO_INCREMENTAL=0` in CI and keep `sccache` enabled.
4. If needed, introduce `just verify-ci` for CI-only smoke behavior.

**Green Phase (required)**

Command: `scripts/install-bats.sh >/dev/null && "$(scripts/install-bats.sh)" scripts/tests/test-fuzz-gating.bats scripts/tests/test-justfile-targets.bats`
Expected: CI-overlap and env-setting shell coverage passes.

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Targeted shell tests passing

### Task 4: Capture timing deltas and operator docs

**Files:**

- Modify: `docs/tasks/README.md`
- Modify: `docs/tasks/tasks.csv`
- Modify: `CHANGELOG.md`
- Optionally modify: prior workflow-hardening docs to cross-reference the runtime-optimization slice

**Preconditions**

- Tasks 1 through 3 are complete.

**Invariants**

- Docs describe the actual split between per-push and nightly verification.
- Timing evidence remains tied to specific workflow run IDs.

**Postconditions**

- Operator docs explain the new workflow ownership boundaries.
- Changelog and task registry reflect the completed runtime-optimization slice.
- Post-rollout timing evidence is captured against run `23010870993`.

**Tests (must exist before implementation)**

Unit:
- `docs_describe_policy_checks_vs_nightly_split`

Invariant:
- `scripts/doc-lint.sh --changed --strict-new`
- `scripts/check-tasks-registry.sh`

Integration:
- `scripts/check-tasks-sync.sh --changed`
- `scripts/check-fast-feedback.sh`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: docs do not yet describe the new per-push vs nightly split.

**Implementation Steps**

1. Update operator docs for pre-install gating and nightly workflow ownership.
2. Update task registry and changelog.
3. Record before/after timing deltas with exact run IDs.

**Green Phase (required)**

Command: `scripts/doc-lint.sh --changed --strict-new && scripts/check-tasks-registry.sh && scripts/check-tasks-sync.sh --changed && scripts/check-fast-feedback.sh`
Expected: docs, task sync, and fast-feedback pass.

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Verification commands passing
