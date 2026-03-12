#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
}

@test "dockerfile uses required builder and runtime bases" {
  run rg '^FROM rust:1\.94-slim AS builder$' "$ROOT/Dockerfile"
  [ "$status" -eq 0 ]

  run rg '^FROM debian:trixie-slim AS base$' "$ROOT/Dockerfile"
  [ "$status" -eq 0 ]

  run rg '^FROM builder AS test$' "$ROOT/Dockerfile"
  [ "$status" -eq 0 ]

  run rg '^FROM base AS runtime$' "$ROOT/Dockerfile"
  [ "$status" -eq 0 ]
}

@test "dockerfile builds and ships required sharo binaries" {
  run rg 'cargo build --release -p sharo-cli -p sharo-daemon -p sharo-tui -p sharo-hazel-mcp' "$ROOT/Dockerfile"
  [ "$status" -eq 0 ]

  run rg 'cargo nextest run --workspace' "$ROOT/Dockerfile"
  [ "$status" -eq 0 ]

  run rg 'COPY --from=builder .*/target/release/sharo /usr/local/bin/sharo' "$ROOT/Dockerfile"
  [ "$status" -eq 0 ]

  run rg 'COPY --from=builder .*/target/release/sharo-daemon /usr/local/bin/sharo-daemon' "$ROOT/Dockerfile"
  [ "$status" -eq 0 ]

  run rg 'COPY --from=builder .*/target/release/sharo-tui /usr/local/bin/sharo-tui' "$ROOT/Dockerfile"
  [ "$status" -eq 0 ]

  run rg 'COPY --from=builder .*/target/release/sharo-hazel-mcp /usr/local/bin/sharo-hazel-mcp' "$ROOT/Dockerfile"
  [ "$status" -eq 0 ]
}

@test "docker helper scripts expose build and smoke procedures" {
  run rg '^Usage:' "$ROOT/scripts/docker-build.sh"
  [ "$status" -eq 0 ]

  run rg -- '--target test' "$ROOT/scripts/docker-build.sh"
  [ "$status" -eq 0 ]

  run rg '^Usage:' "$ROOT/scripts/docker-smoke.sh"
  [ "$status" -eq 0 ]

  run rg 'sharo hazel status' "$ROOT/scripts/docker-smoke.sh"
  [ "$status" -eq 0 ]
}

@test "docker docs cover runtime process and devops runbook" {
  run rg 'root \[`Dockerfile`\]' "$ROOT/docs/docker-runtime-image.md"
  [ "$status" -eq 0 ]

  run rg 'docs/devops/docker-runtime-operations\.md' "$ROOT/README.md"
  [ "$status" -eq 0 ]

  run rg 'CI / Release Procedure' "$ROOT/docs/devops/docker-runtime-operations.md"
  [ "$status" -eq 0 ]
}

@test "justfile includes docker workflow targets" {
  run rg '^docker-build:\s*$' "$ROOT/justfile"
  [ "$status" -eq 0 ]

  run rg '^docker-smoke:\s*$' "$ROOT/justfile"
  [ "$status" -eq 0 ]
}

@test "gitignore covers local docker build artifacts" {
  run rg '^/\.buildx-cache$' "$ROOT/.gitignore"
  [ "$status" -eq 0 ]

  run rg '^/\.docker-buildx$' "$ROOT/.gitignore"
  [ "$status" -eq 0 ]

  run rg '^/docker-data$' "$ROOT/.gitignore"
  [ "$status" -eq 0 ]
}
