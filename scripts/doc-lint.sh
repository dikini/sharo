#!/usr/bin/env bash
set -euo pipefail

: <<'DOC_LINT_POLICY'
Doc lint policy

Scope:
- Lightweight and low-dependency by design (bash + common shell tools).
- Canonical lint scope is repo-local docs only:
  - docs/**/*.md
  - AGENTS.md

Rule classes:
1) Evergreen rules
   - Structural invariants that should not regress (e.g., broken local links).

2) Temporary regression guards
   - Added only for known incidents with high impact or repeat risk.
   - Must include metadata:
     - TEMP_GUARD id
     - reason
     - added (YYYY-MM-DD)
     - review_by (YYYY-MM-DD)
   - Should be removed or converted to evergreen on review.
DOC_LINT_POLICY

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

failures=0

fail() {
  echo "doc-lint: $1" >&2
  failures=$((failures + 1))
}

# Build file scope.
files=()
while IFS= read -r f; do
  files+=("$f")
done < <(find docs -type f -name '*.md' | sort)

if [[ -f "AGENTS.md" ]]; then
  files+=("AGENTS.md")
fi

if [[ "${#files[@]}" -eq 0 ]]; then
  echo "doc-lint: no markdown files in scope"
  exit 0
fi

# -----------------------------------------------------------------------------
# Temporary regression guards
# -----------------------------------------------------------------------------

# TEMP_GUARD: stale_mvp_spec_path
# reason: prior rename from docs/plan/mvp.md to docs/specs/mvp.md caused stale refs.
# added: 2026-03-04
# review_by: 2026-04-15
if rg -n "docs/plan/mvp\.md" "${files[@]}" >/dev/null 2>&1; then
  fail "found stale path 'docs/plan/mvp.md' (use 'docs/specs/mvp.md')"
fi

# -----------------------------------------------------------------------------
# Evergreen checks
# -----------------------------------------------------------------------------

# Local markdown links should resolve.
# - Skip external URLs and anchor-only links.
# - For relative links, resolve from file directory.
# - For absolute paths, accept only if path exists.
while IFS= read -r match; do
  file="${match%%:*}"
  rest="${match#*:}"
  link="$(echo "$rest" | sed -E 's/.*\[[^]]+\]\(([^)]+)\).*/\1/')"

  # Drop optional title part: path "title"
  link="${link%% \"*}"

  # Strip anchor fragment.
  path="${link%%#*}"

  case "$link" in
    http://*|https://*|mailto:*|\#*)
      continue
      ;;
  esac

  # Empty path after stripping anchor means anchor-only link.
  if [[ -z "$path" ]]; then
    continue
  fi

  if [[ "$path" = /* ]]; then
    target="$path"
  else
    target="$(cd "$(dirname "$file")" && realpath -m "$path")"
  fi

  if [[ ! -e "$target" ]]; then
    fail "broken local link in '$file' -> '$link'"
  fi
done < <(rg -n -o '\[[^]]+\]\(([^)]+)\)' "${files[@]}")

if [[ "$failures" -gt 0 ]]; then
  echo "doc-lint: FAILED (${failures} issue(s))" >&2
  exit 1
fi

echo "doc-lint: OK"
