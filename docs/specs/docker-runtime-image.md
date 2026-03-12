# Docker Runtime Image

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Define tests before implementation work.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Invariant, and Integration checks.
> - Use Property-based tests only when a generative framework is actually used.
> - Run `scripts/doc-lint.sh` before commit.

Updated: 2026-03-12
Status: active
Owner: tooling
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-DOCKER-RUNTIME-SPEC-001

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. This spec's task contracts and invariants.
4. In-task updates recorded explicitly in this document.

## Output Contract

- Provide one multi-stage Docker build rooted at the repository root.
- Use `rust:1.94-slim` as the builder image.
- Use `debian:trixie-slim` as the runtime base image.
- Produce a runnable runtime image, not only extracted binaries.

## Evidence / Verification Contract

- Every completion claim must cite verification commands/results in `## Verification`.
- If Docker execution cannot run locally, shell-contract tests and repo verification must still pass.

## Model Compatibility Notes

- The runtime image is daemon-first because `sharo` and `sharo-tui` depend on the daemon socket contract.
- `sharo-cli` is the crate name; the shipped executable remains `sharo`.

## Purpose

Define the canonical container packaging for Sharo so operators can build one runtime image that includes the CLI, daemon, TUI, and Hazel MCP binary with documented local build and smoke procedures.

## Scope

### In Scope

- Root `Dockerfile` with `builder`, `test`, `base`, and `runtime` stages.
- Runtime image contents:
  - `sharo`
  - `sharo-daemon`
  - `sharo-tui`
  - `sharo-hazel-mcp`
- Container helper procedures for image build and smoke verification.
- README and operator-facing usage docs for local container workflows.

### Out of Scope

- Compose, Kubernetes, or orchestration manifests.
- Cross-compilation or multi-architecture publishing.
- Separate per-binary final images.
- Remote registry publishing automation.

## Interfaces / Contracts

- The runtime image MUST start `sharo-daemon` by default.
- The runtime image MUST keep `sharo`, `sharo-tui`, and `sharo-hazel-mcp` available on `PATH`.
- The Docker build MUST include a dedicated test stage that runs the required Rust verification commands before the runtime image is considered valid.
- `sharo-cli` MUST be documented as the `sharo` binary, not as a separate executable.
- The runtime image MUST run as a non-root user.
- Runtime state MUST be stored outside the image layer, with a writable path for persisted daemon state.
- The default daemon socket path MUST remain compatible with the current CLI/TUI default of `/tmp/sharo-daemon.sock`.

## Invariants

- The container story remains one image with one default operational entrypoint.
- The runtime image stays slim by excluding source, Cargo caches, and test artifacts.
- Docker procedures remain additive and do not change existing host workflows.

## Task Contracts

### Task 1: Define the multi-stage build contract

**Preconditions**

- The Rust workspace already builds the requested binaries.

**Invariants**

- Builder and runtime base images remain pinned to the required tags.
- The runtime image remains separate from the test stage.

**Postconditions**

- The spec defines required stages, shipped binaries, and default runtime behavior.

**Tests (must exist before implementation)**

Unit:
- `dockerfile_uses_required_builder_and_runtime_bases`

Invariant:
- `dockerfile_builds_and_ships_required_sharo_binaries`

Integration:
- `docker_helper_scripts_expose_build_and_smoke_procedures`

Property-based (optional):
- not applicable

### Task 2: Define operator procedures

**Preconditions**

- The runtime image contract is fixed.

**Invariants**

- Procedures document one runnable stack image.
- Help and smoke commands match the shipped binaries.

**Postconditions**

- The repo documents build, smoke, daemon, CLI, and TUI container procedures.

**Tests (must exist before implementation)**

Unit:
- `justfile_includes_docker_workflow_targets`

Invariant:
- `docker_readme_documents_runtime_stack_not_toolbox_only`

Integration:
- `docker_smoke_reaches_daemon_control_plane`

Property-based (optional):
- not applicable

## Verification

- `scripts/install-bats.sh >/dev/null && "$(scripts/install-bats.sh)" scripts/tests/test-docker-runtime-image.bats`
- `scripts/doc-lint.sh --changed --strict-new`
- `scripts/check-tasks-registry.sh`
- `scripts/check-tasks-sync.sh --changed`
