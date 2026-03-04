#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
CSV="docs/tasks/tasks.csv"

usage() {
  cat <<'USAGE'
Usage:
  scripts/tasks.sh
  scripts/tasks.sh --status <planned|deferred|in_progress|done|cancelled>
  scripts/tasks.sh --summary
USAGE
}

status_filter=""
summary=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --status)
      shift
      [[ $# -gt 0 ]] || { echo "tasks: --status requires a value" >&2; exit 2; }
      status_filter="$1"
      shift
      ;;
    --summary)
      summary=true
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "tasks: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

[[ -f "$CSV" ]] || { echo "tasks: registry not found: $CSV" >&2; exit 1; }

if [[ "$summary" == true ]]; then
  awk -F',' 'NR>1{count[$5]++} END{for (s in count) printf "%s,%d\n", s, count[s]}' "$CSV" | sort
  exit 0
fi

if [[ -n "$status_filter" ]]; then
  awk -F',' -v s="$status_filter" 'NR==1 || $5==s' "$CSV"
else
  cat "$CSV"
fi
