#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

image_tag="sharo:local"
container_tool="${CONTAINER_TOOL:-}"
dockerfile="$ROOT/Dockerfile"
skip_test=false

usage() {
  cat <<'USAGE'
Usage:
  scripts/docker-build.sh
  scripts/docker-build.sh --tag <image-tag>
  scripts/docker-build.sh --container-tool <docker|podman>
  scripts/docker-build.sh --dockerfile <path>
  scripts/docker-build.sh --skip-test
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --tag)
      shift
      [[ $# -gt 0 ]] || {
        echo "docker-build: --tag requires a value" >&2
        exit 2
      }
      image_tag="$1"
      shift
      ;;
    --container-tool)
      shift
      [[ $# -gt 0 ]] || {
        echo "docker-build: --container-tool requires a value" >&2
        exit 2
      }
      container_tool="$1"
      shift
      ;;
    --dockerfile)
      shift
      [[ $# -gt 0 ]] || {
        echo "docker-build: --dockerfile requires a value" >&2
        exit 2
      }
      dockerfile="$1"
      shift
      ;;
    --skip-test)
      skip_test=true
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "docker-build: unknown argument '$1'" >&2
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
    echo "docker-build: missing container tool 'docker' or 'podman'" >&2
    exit 1
  fi
fi

if [[ "$skip_test" != true ]]; then
  echo "docker-build: building test stage with $container_tool"
  "$container_tool" build -f "$dockerfile" --target test "$ROOT"
fi

echo "docker-build: building runtime image $image_tag with $container_tool"
"$container_tool" build -f "$dockerfile" --target runtime -t "$image_tag" "$ROOT"

echo "docker-build: built $image_tag"
