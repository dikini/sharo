# sharo

`sharo` is a Rust workspace with six crates:

- `sharo-core`: shared core logic and protocol/model connector surface.
- `sharo-cli`: command-line interface.
- `sharo-daemon`: daemon/runtime process for task handling.
- `sharo-tui`: terminal user interface over the daemon control plane.
- `sharo-hazel-core`: structured-memory canonical contracts, lifecycle, and ingestion/sleep interface validators.
- `sharo-hazel-mcp`: stdio-first MCP wrapper for Hazel schema compatibility and recollection normalization.

## Docker Runtime Image

The repository includes a daemon-first multi-stage Docker image that builds on `rust:1.94-slim` and runs on `debian:trixie-slim`.

- Build and test the image: `scripts/docker-build.sh --tag sharo:local`
- Smoke-check the built image: `scripts/docker-smoke.sh --image sharo:local`
- Read the operator procedure guide: [`docs/docker-runtime-image.md`](docs/docker-runtime-image.md)
- Read the devops runbook: [`docs/devops/docker-runtime-operations.md`](docs/devops/docker-runtime-operations.md)

## Status

This project is actively in development.

Behavior, APIs, CLI output, and persisted state can change without notice.
Breakages are expected while core functionality and workflows are still evolving.
