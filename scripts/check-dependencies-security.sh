#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

strict_mode=true

usage() {
  cat <<'USAGE'
Usage:
  scripts/check-dependencies-security.sh
  scripts/check-dependencies-security.sh --warn-only
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --warn-only)
      strict_mode=false
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "dependency-security: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

need_tool() {
  local command="$1"
  local label="$2"
  if $command >/dev/null 2>&1; then
    return 0
  fi
  if [[ "$strict_mode" == true ]]; then
    echo "dependency-security: required tool missing: $label" >&2
    exit 1
  fi
  echo "dependency-security: warning: $label not available; skipping check"
  return 1
}

if need_tool "cargo deny --version" "cargo-deny"; then
  cargo deny check
fi

if need_tool "cargo audit --version" "cargo-audit"; then
  cargo audit
fi

echo "dependency-security: OK"
