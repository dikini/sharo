# Docker Runtime Image Implementation Plan

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Invariant, and Integration checks.
> - Use Property-based tests only when a generative framework is actually used.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: add one multi-stage Docker runtime image and operator procedures for building and smoke-checking it locally.
Architecture: keep packaging additive to the existing workspace by introducing a root Dockerfile, shell helper procedures, and operator docs. Make the runtime image daemon-first so the CLI and TUI remain usable inside the same container.
Tech Stack: Dockerfile multi-stage build, Rust 2024 workspace binaries, Bash helper scripts, Bats shell tests, existing README/docs.
Template-Profile: tdd-strict-v1
Updated: 2026-03-12
Status: completed

Task-Registry-Refs: TASK-DOCKER-RUNTIME-PLAN-001, TASK-DOCKER-RUNTIME-SPEC-001

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. This plan's execution and verification contracts.
4. Explicit plan updates recorded in this file.

## Output Contract

- Add the Docker packaging surface without changing existing non-container workflows.
- Preserve daemon-first runtime behavior.
- Keep verification concrete and runnable through repo shell tests plus fast-feedback.

## Task Update Contract

- New container workflow behavior must remain additive to existing host workflows.
- Runtime packaging changes must preserve the daemon-first image contract and shipped binary set.

## Model Compatibility Notes

- `sharo-cli` is implemented as the `sharo` binary and must be described that way in image docs.
- The container defaults intentionally preserve `/tmp/sharo-daemon.sock` so existing CLI/TUI defaults remain valid.

## Execution Mode

- Mode: execute-with-checkpoints
- Default: execute-with-checkpoints unless user explicitly requests plan-only output.

## Completion Gate

- Docker shell-contract tests pass.
- Docs/task/changelog updates are in sync.
- `scripts/check-fast-feedback.sh` passes on the final tree.

### Task 1: Add Docker packaging contracts

**Files:**

- Create: `Dockerfile`
- Create: `.dockerignore`
- Create: `scripts/tests/test-docker-runtime-image.bats`

**Preconditions**

- The workspace builds `sharo`, `sharo-daemon`, `sharo-tui`, and `sharo-hazel-mcp`.

**Invariants**

- Builder base stays `rust:1.94-slim`.
- Runtime base stays `debian:trixie-slim`.

**Postconditions**

- Docker packaging stages and shipped binaries are pinned by shell tests.

**Tests (must exist before implementation)**

Unit:
- `dockerfile_uses_required_builder_and_runtime_bases`

Invariant:
- `dockerfile_builds_and_ships_required_sharo_binaries`

Integration:
- `docker_helper_scripts_expose_build_and_smoke_procedures`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `scripts/install-bats.sh >/dev/null && "$(scripts/install-bats.sh)" scripts/tests/test-docker-runtime-image.bats`
Expected: fails because the Dockerfile, helper scripts, and `just` targets do not exist yet.

**Implementation Steps**

1. Add the shell-contract Bats test.
2. Add `.dockerignore` and the multi-stage Dockerfile.
3. Keep the runtime stage daemon-first and ship the required binaries.

**Green Phase (required)**

Command: `scripts/install-bats.sh >/dev/null && "$(scripts/install-bats.sh)" scripts/tests/test-docker-runtime-image.bats`
Expected: all Docker packaging shell-contract tests pass.

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Docker packaging shell-contract tests passing

**Verification**

- `scripts/install-bats.sh >/dev/null && "$(scripts/install-bats.sh)" scripts/tests/test-docker-runtime-image.bats`

### Task 2: Add operator procedures

**Files:**

- Create: `scripts/docker-build.sh`
- Create: `scripts/docker-smoke.sh`
- Modify: `justfile`
- Modify: `README.md`

**Preconditions**

- Dockerfile contract is fixed.

**Invariants**

- The default image remains daemon-first.
- Procedures stay explicit about `sharo-cli` mapping to `sharo`.

**Postconditions**

- Operators have scripted build and smoke procedures plus simple `just` wrappers.

**Tests (must exist before implementation)**

Unit:
- `justfile_includes_docker_workflow_targets`

Invariant:
- `docker_helper_scripts_expose_build_and_smoke_procedures`

Integration:
- `docker_readme_documents_runtime_stack_not_toolbox_only`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `scripts/install-bats.sh >/dev/null && "$(scripts/install-bats.sh)" scripts/tests/test-docker-runtime-image.bats`
Expected: fails because `scripts/docker-build.sh`, `scripts/docker-smoke.sh`, and `just` docker targets do not exist yet.

**Implementation Steps**

1. Add `scripts/docker-build.sh` with test-stage and runtime-stage build flow.
2. Add `scripts/docker-smoke.sh` with help-surface and daemon-control-plane smoke checks.
3. Add `just docker-build` and `just docker-smoke`.
4. Update README and operator docs with build and run procedures.

**Green Phase (required)**

Command: `scripts/check-shell-quality.sh --changed && scripts/install-bats.sh >/dev/null && "$(scripts/install-bats.sh)" scripts/tests/test-docker-runtime-image.bats`
Expected: shell formatting/lint and Docker shell-contract tests pass.

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Shell quality and Docker shell-contract tests passing

**Verification**

- `scripts/check-shell-quality.sh --changed`
- `scripts/install-bats.sh >/dev/null && "$(scripts/install-bats.sh)" scripts/tests/test-docker-runtime-image.bats`

### Task 3: Add tracking and completion docs

**Files:**

- Modify: `docs/tasks/tasks.csv`
- Modify: `CHANGELOG.md`
- Create: `docs/specs/docker-runtime-image.md`
- Create: `docs/plans/2026-03-12-docker-runtime-image-plan.md`

**Preconditions**

- Implementation scope is settled.

**Invariants**

- Task registry source paths remain valid.
- Changelog reflects the completed packaging work.

**Postconditions**

- Docker runtime packaging is tracked as completed canonical work.

**Tests (must exist before implementation)**

Unit:
- `docs_reference_docker_runtime_packaging_contract`

Invariant:
- `tasks_registry_references_docker_runtime_spec_and_plan`

Integration:
- `docs_and_tasks_pass_strict_repo_checks`

Property-based (optional):
- not applicable

**Red Phase (required before code changes)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: fails until the new spec/plan docs contain strict-profile sections and task rows are synced.

**Implementation Steps**

1. Add the Docker runtime spec and implementation plan.
2. Add task-registry rows for the spec and plan.
3. Update the changelog with the completed packaging slice.

**Green Phase (required)**

Command: `scripts/doc-lint.sh --changed --strict-new && scripts/check-tasks-registry.sh && scripts/check-tasks-sync.sh --changed`
Expected: strict docs and task checks pass.

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Docs, task registry, and sync checks passing

**Verification**

- `scripts/doc-lint.sh --changed --strict-new && scripts/check-tasks-registry.sh && scripts/check-tasks-sync.sh --changed`
- `scripts/check-fast-feedback.sh`
