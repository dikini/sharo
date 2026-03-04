#!/usr/bin/env bash
set -euo pipefail

: <<'DOC_LINT_POLICY'
Doc lint policy

Scope:
- Lightweight and low-dependency by design (bash + common shell tools).
- Canonical lint scope is repo-local docs only:
  - docs/**/*.md
  - AGENTS.md

Usage:
- scripts/doc-lint.sh                 # full lint scope
- scripts/doc-lint.sh --changed       # changed/untracked docs only
- scripts/doc-lint.sh --path <file>   # one file
- scripts/doc-lint.sh --strict-new    # enforce strict profile on new specs/plans

Rule classes:
1) Evergreen rules
   - Structural invariants that should not regress (e.g., broken local links).
   - For documents with `Template-Profile: tdd-strict-v1`, enforce strict TDD
     section shape and ordering constraints.

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

mode="all"
target_path=""
strict_new=false

usage() {
  cat <<'EOF'
Usage: scripts/doc-lint.sh [--changed] [--path <file>] [--strict-new]
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --changed)
      mode="changed"
      shift
      ;;
    --path)
      mode="path"
      shift
      if [[ $# -eq 0 ]]; then
        echo "doc-lint: --path requires a value" >&2
        exit 2
      fi
      target_path="$1"
      shift
      ;;
    --strict-new)
      strict_new=true
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "doc-lint: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

failures=0

fail() {
  echo "doc-lint: $1" >&2
  failures=$((failures + 1))
}

is_in_scope_file() {
  local f="$1"
  [[ "$f" == AGENTS.md || "$f" == docs/*.md || "$f" == docs/*/*.md || "$f" == docs/*/*/*.md || "$f" == docs/*/*/*/*.md ]]
}

collect_files_all() {
  local out=()
  while IFS= read -r f; do out+=("$f"); done < <(find docs -type f -name '*.md' | sort)
  if [[ -f AGENTS.md ]]; then
    out+=("AGENTS.md")
  fi
  printf '%s\n' "${out[@]}"
}

collect_files_changed() {
  {
    git diff --name-only -- docs AGENTS.md
    git diff --cached --name-only -- docs AGENTS.md
    git ls-files --others --exclude-standard -- docs AGENTS.md
  } | sed '/^$/d' | sort -u
}

files=()
case "$mode" in
  all)
    while IFS= read -r f; do
      [[ -z "$f" ]] && continue
      files+=("$f")
    done < <(collect_files_all)
    ;;
  changed)
    while IFS= read -r f; do
      [[ -z "$f" ]] && continue
      [[ -e "$f" ]] || continue
      is_in_scope_file "$f" || continue
      files+=("$f")
    done < <(collect_files_changed)
    ;;
  path)
    if [[ ! -e "$target_path" ]]; then
      fail "path not found: $target_path"
    elif ! is_in_scope_file "$target_path"; then
      fail "path out of lint scope: $target_path"
    else
      files+=("$target_path")
    fi
    ;;
esac

if [[ "${#files[@]}" -eq 0 ]]; then
  echo "doc-lint: no markdown files in scope"
  exit 0
fi

# -----------------------------------------------------------------------------
# Strict checks for NEW specs/plans (opt-in flag)
# -----------------------------------------------------------------------------
if [[ "$strict_new" == "true" ]]; then
  new_docs=()
  while IFS= read -r f; do
    [[ -z "$f" ]] && continue
    new_docs+=("$f")
  done < <({
    git ls-files --others --exclude-standard -- docs/specs docs/plans
    git diff --cached --name-status -- docs/specs docs/plans | awk '$1=="A"{print $2}'
  } | sort -u)

  for nf in "${new_docs[@]}"; do
    [[ -e "$nf" ]] || continue
    [[ "$nf" == *.md ]] || continue
    if [[ "$nf" == docs/specs/* || "$nf" == docs/plans/* ]]; then
      if ! rg -n "^Template-Profile:\\s*tdd-strict-v1\\s*$" "$nf" >/dev/null 2>&1; then
        fail "$nf is new and must include 'Template-Profile: tdd-strict-v1'"
      fi
    fi
  done
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

# Strict TDD template profile checks.
strict_files=()
while IFS= read -r sf; do strict_files+=("$sf"); done < <(rg -l "^Template-Profile:\\s*tdd-strict-v1\\s*$" "${files[@]}" || true)

has_line() {
  local pattern="$1"
  local file="$2"
  rg -n "$pattern" "$file" >/dev/null 2>&1
}

line_no() {
  local pattern="$1"
  local file="$2"
  rg -n "$pattern" "$file" | head -n1 | cut -d: -f1
}

for sf in "${strict_files[@]}"; do
  has_line "^\\*\\*Preconditions\\*\\*$" "$sf" || fail "$sf missing '**Preconditions**'"
  has_line "^\\*\\*Invariants\\*\\*$" "$sf" || fail "$sf missing '**Invariants**'"
  has_line "^\\*\\*Postconditions\\*\\*$" "$sf" || fail "$sf missing '**Postconditions**'"
  has_line "^\\*\\*Tests \\(must exist before implementation\\)\\*\\*$" "$sf" || fail "$sf missing strict tests heading"
  has_line "^Unit:$" "$sf" || fail "$sf missing 'Unit:' section"
  has_line "^Property:$" "$sf" || fail "$sf missing 'Property:' section"
  has_line "^Integration:$" "$sf" || fail "$sf missing 'Integration:' section"

  if [[ "$sf" == docs/plans/* || "$sf" == *"/plan.template.md" ]]; then
    has_line "^\\*\\*Red Phase \\(required before code changes\\)\\*\\*$" "$sf" || fail "$sf missing Red Phase section"
    has_line "^\\*\\*Implementation Steps\\*\\*$" "$sf" || fail "$sf missing Implementation Steps section"
    has_line "^\\*\\*Green Phase \\(required\\)\\*\\*$" "$sf" || fail "$sf missing Green Phase section"
    has_line "^\\*\\*Completion Evidence\\*\\*$" "$sf" || fail "$sf missing Completion Evidence section"

    red_ln="$(line_no "^\\*\\*Red Phase \\(required before code changes\\)\\*\\*$" "$sf" || true)"
    impl_ln="$(line_no "^\\*\\*Implementation Steps\\*\\*$" "$sf" || true)"
    green_ln="$(line_no "^\\*\\*Green Phase \\(required\\)\\*\\*$" "$sf" || true)"
    tests_ln="$(line_no "^\\*\\*Tests \\(must exist before implementation\\)\\*\\*$" "$sf" || true)"

    if [[ -n "$tests_ln" && -n "$impl_ln" && "$tests_ln" -gt "$impl_ln" ]]; then
      fail "$sf has Tests section after Implementation Steps"
    fi
    if [[ -n "$red_ln" && -n "$impl_ln" && "$red_ln" -gt "$impl_ln" ]]; then
      fail "$sf has Red Phase after Implementation Steps"
    fi
    if [[ -n "$impl_ln" && -n "$green_ln" && "$impl_ln" -gt "$green_ln" ]]; then
      fail "$sf has Implementation Steps after Green Phase"
    fi
  fi
done

# Template self-checks (always enforced).
if [[ -f "docs/templates/plan.template.md" ]]; then
  has_line "^Template-Profile:\\s*tdd-strict-v1\\s*$" "docs/templates/plan.template.md" || fail "docs/templates/plan.template.md missing strict profile marker"
fi
if [[ -f "docs/templates/spec.template.md" ]]; then
  has_line "^Template-Profile:\\s*tdd-strict-v1\\s*$" "docs/templates/spec.template.md" || fail "docs/templates/spec.template.md missing strict profile marker"
fi
if [[ -f "docs/templates/CHANGELOG.template.md" ]]; then
  has_line "^## Unreleased$" "docs/templates/CHANGELOG.template.md" || fail "docs/templates/CHANGELOG.template.md missing '## Unreleased'"
fi

if [[ "$failures" -gt 0 ]]; then
  echo "doc-lint: FAILED (${failures} issue(s))" >&2
  exit 1
fi

echo "doc-lint: OK"
