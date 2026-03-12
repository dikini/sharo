# Docker Runtime Operations

This runbook covers local and CI/container operations for the daemon-first `sharo` image.

## Build Procedure

Build the validation stage and then the runtime image:

```bash
scripts/docker-build.sh --tag sharo:local
```

To skip the explicit `test` target after a separate verified build:

```bash
scripts/docker-build.sh --tag sharo:local --skip-test
```

Equivalent `just` targets:

```bash
just docker-build
just docker-smoke
```

## Image Contract

- Builder base: `rust:1.94-slim`
- Runtime base: `debian:trixie-slim`
- Default entrypoint: `sharo-daemon`
- Default command:

```bash
sharo-daemon start --socket-path /tmp/sharo-daemon.sock --store-path /var/lib/sharo/daemon-store.json
```

- Shipped binaries:
  - `sharo`
  - `sharo-daemon`
  - `sharo-tui`
  - `sharo-hazel-mcp`

`sharo-cli` is the crate name. The executable in the image is `sharo`.

## Runtime Procedure

Start the daemon with persisted state:

```bash
docker run -d --rm \
  --name sharo \
  -v sharo-data:/var/lib/sharo \
  sharo:local
```

Check control-plane reachability:

```bash
docker exec sharo sharo hazel status
```

Run the TUI against the in-container daemon:

```bash
docker exec -it sharo sharo-tui
```

Inspect the packaged binaries directly:

```bash
docker run --rm --entrypoint sharo sharo:local --help
docker run --rm --entrypoint sharo-daemon sharo:local --help
docker run --rm --entrypoint sharo-tui sharo:local --help
docker run --rm --entrypoint sharo-hazel-mcp sharo:local --help
```

## CI / Release Procedure

- Build with `scripts/docker-build.sh --tag <image>`
- Smoke-test with `scripts/docker-smoke.sh --image <image>`
- Push only after the Docker smoke procedure and `scripts/check-fast-feedback.sh` are both green
- Keep the image daemon-first; do not switch the default entrypoint to `sharo-tui`

## State And Paths

- The daemon socket stays at `/tmp/sharo-daemon.sock` for CLI/TUI compatibility
- Persistent daemon state lives under `/var/lib/sharo`
- The runtime image is non-root, so bind mounts must be writable by the in-container user or by world/group policy on the host

## Local Artifact Hygiene

These local-only artifacts are ignored by Git and should stay out of commits:

- `target/`
- `.buildx-cache/`
- `.docker-buildx/`
- `docker-data/`

Use named volumes or ignored local directories for container state. Do not add exported runtime state or local build caches to the repository.

## Troubleshooting

- If the smoke test cannot reach the daemon, inspect container logs with `docker logs sharo`
- If `sharo-tui` fails to render, verify you started it with `docker exec -it`
- If the build environment lacks Docker, `scripts/docker-build.sh` can also target Podman via `--container-tool podman`
