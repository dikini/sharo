#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

mode="changed"
write_marker=true
git_dir="$(git rev-parse --git-dir)"
marker_file="$git_dir/.fast-feedback.ok"

usage() {
  cat <<'USAGE'
Usage:
  scripts/check-fast-feedback.sh
  scripts/check-fast-feedback.sh --changed
  scripts/check-fast-feedback.sh --all
  scripts/check-fast-feedback.sh --no-marker
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --changed)
      mode="changed"
      shift
      ;;
    --all)
      mode="all"
      shift
      ;;
    --no-marker)
      write_marker=false
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "fast-feedback: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

status_sha() {
  git status --porcelain=v1 --untracked-files=all | sha256sum | awk '{print $1}'
}

scripts/doc-lint.sh --changed --strict-new
scripts/check-doc-terms.sh --changed
scripts/check-tasks-registry.sh
scripts/check-tasks-sync.sh --changed
scripts/check-rust-policy.sh
if [[ "$mode" == "all" ]]; then
  scripts/check-rust-tests.sh --all
else
  scripts/check-rust-tests.sh --changed
fi
scripts/check-sync-manifest.sh --changed
scripts/check-knot-diff.sh --mapping docs/tasks/knot-diff-mapping.csv
scripts/check-research-references.sh --registry docs/tasks/research-reference-rules.csv
if [[ "$mode" == "all" ]]; then
  scripts/run-shell-tests.sh --all
else
  scripts/run-shell-tests.sh --changed
fi

if [[ "$write_marker" == true ]]; then
  {
    echo "timestamp_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "head=$(git rev-parse HEAD)"
    echo "status_sha=$(status_sha)"
  } > "$marker_file"
  echo "fast-feedback: marker updated at $marker_file"
fi

echo "fast-feedback: OK"
