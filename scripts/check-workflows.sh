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

actionlint_bin=""
if [[ -x "$ROOT/.tools/actionlint/actionlint" ]]; then
  actionlint_bin="$ROOT/.tools/actionlint/actionlint"
elif command -v actionlint >/dev/null 2>&1; then
  actionlint_bin="$(command -v actionlint)"
fi

if [[ -z "$actionlint_bin" ]]; then
  if [[ "$warn_missing" == true ]]; then
    echo "workflow-lint: warning: actionlint missing; skipping"
    exit 0
  fi
  echo "workflow-lint: missing required tool 'actionlint'" >&2
  echo "workflow-lint: install hint: scripts/bootstrap-dev.sh --apply (installs local binary in .tools/actionlint)" >&2
  exit 1
fi

"$actionlint_bin"
echo "workflow-lint: OK"
