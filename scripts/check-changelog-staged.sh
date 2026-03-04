#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

# If nothing is staged, nothing to enforce.
staged_any="$(git diff --cached --name-only)"
if [[ -z "$staged_any" ]]; then
  exit 0
fi

# Allow commit only when CHANGELOG.md is staged as well.
if git diff --cached --name-only -- CHANGELOG.md | grep -q '^CHANGELOG.md$'; then
  exit 0
fi

cat >&2 <<'EOF'
pre-commit policy check failed:
  CHANGELOG.md must be updated and staged for every task-completion commit.
EOF
exit 1
