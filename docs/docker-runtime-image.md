# Docker Runtime Image

The repository now ships one daemon-first runtime image.

## Build

Build and verify the Docker test stage, then build the runtime image:

```bash
scripts/docker-build.sh --tag sharo:local
```

Equivalent `just` shortcut:

```bash
just docker-build
```

Local Docker and BuildKit cache directories remain non-canonical. The repository `.gitignore` excludes common local build artifacts such as `.buildx-cache/`, `.docker-buildx/`, `docker-data/`, and the Rust `target/` tree.

## Smoke

Run the container smoke checks against an already-built image:

```bash
scripts/docker-smoke.sh --image sharo:local
```

Equivalent `just` shortcut:

```bash
just docker-smoke
```

## Dockerfile

The canonical image definition is the root [`Dockerfile`](/home/dikini/Projects/sharo/Dockerfile).

Build stages:

- `builder`: compiles release binaries from `rust:1.94-slim`
- `test`: runs the required workspace verification before a runtime image is considered valid
- `base`: provisions the slim Debian runtime foundation
- `runtime`: ships only the final executables on top of `debian:trixie-slim`

## Run

Start the daemon-first image with persisted state:

```bash
docker run -d --rm --name sharo -v sharo-data:/var/lib/sharo sharo:local
```

The default container command is:

```bash
sharo-daemon start --socket-path /tmp/sharo-daemon.sock --store-path /var/lib/sharo/daemon-store.json
```

## Use The Binaries

The image ships these binaries on `PATH`:

- `sharo`
- `sharo-daemon`
- `sharo-tui`
- `sharo-hazel-mcp`

`sharo-cli` is the crate name; the executable is `sharo`.

Run CLI commands against the in-container daemon:

```bash
docker exec sharo sharo hazel status
```

Run the TUI in the same container:

```bash
docker exec -it sharo sharo-tui
```

Inspect command help without starting an extra daemon:

```bash
docker run --rm --entrypoint sharo sharo:local --help
docker run --rm --entrypoint sharo-daemon sharo:local --help
docker run --rm --entrypoint sharo-tui sharo:local --help
docker run --rm --entrypoint sharo-hazel-mcp sharo:local --help
```

## DevOps

Operational procedures, upgrade guidance, and troubleshooting live in [`docs/devops/docker-runtime-operations.md`](/home/dikini/Projects/sharo/docs/devops/docker-runtime-operations.md).
