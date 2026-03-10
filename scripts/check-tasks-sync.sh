#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

mode="changed"
range=""

usage() {
  cat <<'USAGE'
Usage:
  scripts/check-tasks-sync.sh --changed
  scripts/check-tasks-sync.sh --range <git-range>
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --changed)
      mode="changed"
      shift
      ;;
    --range)
      mode="range"
      shift
      [[ $# -gt 0 ]] || {
        echo "tasks-sync: --range requires a value" >&2
        exit 2
      }
      range="$1"
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "tasks-sync: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

changed_files() {
  if [[ "$mode" == "range" ]]; then
    git diff --name-only "$range"
  else
    {
      git diff --name-only
      git diff --cached --name-only
      git ls-files --others --exclude-standard
    } | sort -u
  fi
}

files="$(changed_files | sed '/^$/d' || true)"
[[ -n "$files" ]] || {
  echo "tasks-sync: no changed files in scope"
  exit 0
}

if echo "$files" | rg -n '^(docs/specs/|docs/plans/|scripts/)' >/dev/null 2>&1; then
  if ! echo "$files" | rg -n '^docs/tasks/tasks\.csv$' >/dev/null 2>&1; then
    echo "tasks-sync: docs/tasks/tasks.csv must be updated when docs/specs, docs/plans, or scripts change" >&2
    exit 1
  fi
fi

echo "tasks-sync: OK"
