#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: scripts/check-conventional-commit.sh <commit-msg-file>" >&2
  exit 2
fi

msg_file="$1"
[[ -f "$msg_file" ]] || { echo "commit-msg check: file not found: $msg_file" >&2; exit 2; }

first_line="$(head -n1 "$msg_file")"

# Conventional Commits 1.0.0
# <type>[optional scope][!]: <description>
pattern='^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9._/-]+\))?(!)?: .+'

if [[ "$first_line" =~ $pattern ]]; then
  exit 0
fi

cat >&2 <<'EOF'
commit-msg check failed: message must follow Conventional Commits:
  <type>[optional scope][!]: <description>
Examples:
  feat(parser): add support for xyz
  fix: prevent panic on empty input
EOF
exit 1
