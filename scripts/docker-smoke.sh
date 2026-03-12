#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

image_tag="sharo:local"
container_tool="${CONTAINER_TOOL:-}"
container_name="sharo-smoke-$$"

usage() {
  cat <<'USAGE'
Usage:
  scripts/docker-smoke.sh
  scripts/docker-smoke.sh --image <image-tag>
  scripts/docker-smoke.sh --container-tool <docker|podman>
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --image)
      shift
      [[ $# -gt 0 ]] || {
        echo "docker-smoke: --image requires a value" >&2
        exit 2
      }
      image_tag="$1"
      shift
      ;;
    --container-tool)
      shift
      [[ $# -gt 0 ]] || {
        echo "docker-smoke: --container-tool requires a value" >&2
        exit 2
      }
      container_tool="$1"
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "docker-smoke: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

if [[ -z "$container_tool" ]]; then
  if command -v docker >/dev/null 2>&1; then
    container_tool="docker"
  elif command -v podman >/dev/null 2>&1; then
    container_tool="podman"
  else
    echo "docker-smoke: missing container tool 'docker' or 'podman'" >&2
    exit 1
  fi
fi

cleanup() {
  "$container_tool" rm -f "$container_name" >/dev/null 2>&1 || true
}
trap cleanup EXIT

echo "docker-smoke: verifying command help surfaces"
"$container_tool" run --rm --entrypoint sharo "$image_tag" --help >/dev/null
"$container_tool" run --rm --entrypoint sharo-daemon "$image_tag" --help >/dev/null
"$container_tool" run --rm --entrypoint sharo-tui "$image_tag" --help >/dev/null
"$container_tool" run --rm --entrypoint sharo-hazel-mcp "$image_tag" --help >/dev/null || true

echo "docker-smoke: starting daemon container"
"$container_tool" run -d --rm --name "$container_name" "$image_tag" >/dev/null

ready=false
for _ in $(seq 1 30); do
  if "$container_tool" exec "$container_name" sh -lc 'test -S /tmp/sharo-daemon.sock'; then
    ready=true
    break
  fi
  sleep 1
done

if [[ "$ready" != true ]]; then
  echo "docker-smoke: daemon socket did not become ready" >&2
  exit 1
fi

echo "docker-smoke: verifying CLI reaches daemon"
"$container_tool" exec "$container_name" sharo hazel status >/dev/null

echo "docker-smoke: OK"
