#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

warn_missing=false

usage() {
  cat <<'USAGE'
Usage:
  scripts/check-workflows.sh
  scripts/check-workflows.sh --warn-missing
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --warn-missing)
      warn_missing=true
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "workflow-lint: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

if ! command -v actionlint >/dev/null 2>&1; then
  if [[ "$warn_missing" == true ]]; then
    echo "workflow-lint: warning: actionlint missing; skipping"
    exit 0
  fi
  echo "workflow-lint: missing required tool 'actionlint'" >&2
  echo "workflow-lint: install hint: apt install -y actionlint (or use release binary)" >&2
  exit 1
fi

actionlint
echo "workflow-lint: OK"
