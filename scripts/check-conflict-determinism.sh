#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

# Deterministic allowlist for high-churn files when unresolved conflicts are present.
allowlist_regex='^(CHANGELOG\.md|docs/tasks/tasks\.csv|Cargo\.lock)$'

if rg -n '^(<<<<<<<|=======|>>>>>>>)' -- . >/dev/null 2>&1; then
  echo "conflict-determinism: unresolved conflict markers detected" >&2
  exit 1
fi

mapfile -t unmerged < <(git diff --name-only --diff-filter=U | sed '/^$/d' | sort -u)
if [[ "${#unmerged[@]}" -eq 0 ]]; then
  echo "conflict-determinism: OK (no unmerged paths)"
  exit 0
fi

violations=0
for p in "${unmerged[@]}"; do
  if [[ ! "$p" =~ $allowlist_regex ]]; then
    echo "conflict-determinism: non-policy unmerged path: $p" >&2
    violations=1
  fi
done

if [[ "$violations" -ne 0 ]]; then
  exit 1
fi

echo "conflict-determinism: OK"
