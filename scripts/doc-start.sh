#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

usage() {
  cat <<'EOF'
Usage:
  scripts/doc-start.sh spec <slug>
  scripts/doc-start.sh plan <slug>
EOF
}

if [[ $# -ne 2 ]]; then
  usage
  exit 2
fi

kind="$1"
slug="$2"

case "$kind" in
  spec|plan) ;;
  *)
    usage
    exit 2
    ;;
esac

created="$(scripts/doc-new.sh "$kind" "$slug")"
scripts/doc-lint.sh --path "$created" --strict-new

echo "doc-start: created and linted $created"
echo "next: edit the file, then run scripts/doc-lint.sh --changed --strict-new"
