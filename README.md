# sharo

`sharo` is a Rust workspace with five crates:

- `sharo-core`: shared core logic and protocol/model connector surface.
- `sharo-cli`: command-line interface.
- `sharo-daemon`: daemon/runtime process for task handling.
- `sharo-hazel-core`: structured-memory canonical contracts, lifecycle, and ingestion/sleep interface validators.
- `sharo-hazel-mcp`: stdio-first MCP wrapper for Hazel schema compatibility and recollection normalization.

## Status

This project is actively in development.

Behavior, APIs, CLI output, and persisted state can change without notice.
Breakages are expected while core functionality and workflows are still evolving.
