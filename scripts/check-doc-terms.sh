#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

mode="all"
target_path=""
RULES_FILE="docs/terms/terminology.rules"

usage() {
  cat <<'USAGE'
Usage:
  scripts/check-doc-terms.sh
  scripts/check-doc-terms.sh --changed
  scripts/check-doc-terms.sh --path <file>
  scripts/check-doc-terms.sh --rules <rules-file>
USAGE
}

is_in_scope_file() {
  local f="$1"
  [[ "$f" == AGENTS.md || "$f" == docs/*.md || "$f" == docs/*/*.md || "$f" == docs/*/*/*.md || "$f" == docs/*/*/*/*.md ]]
}

collect_all() {
  local out=()
  while IFS= read -r f; do out+=("$f"); done < <(find docs -type f -name '*.md' | sort)
  if [[ -f AGENTS.md ]]; then
    out+=("AGENTS.md")
  fi
  printf '%s\n' "${out[@]}"
}

collect_changed() {
  {
    git diff --name-only -- docs AGENTS.md
    git diff --cached --name-only -- docs AGENTS.md
    git ls-files --others --exclude-standard -- docs AGENTS.md
  } | sed '/^$/d' | sort -u
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
        echo "doc-terms: --path requires a value" >&2
        exit 2
      fi
      target_path="$1"
      shift
      ;;
    --rules)
      shift
      if [[ $# -eq 0 ]]; then
        echo "doc-terms: --rules requires a value" >&2
        exit 2
      fi
      RULES_FILE="$1"
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "doc-terms: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

if [[ ! -f "$RULES_FILE" ]]; then
  echo "doc-terms: rules file not found: $RULES_FILE" >&2
  exit 2
fi

files=()
case "$mode" in
  all)
    while IFS= read -r f; do
      [[ -z "$f" ]] && continue
      files+=("$f")
    done < <(collect_all)
    ;;
  changed)
    while IFS= read -r f; do
      [[ -z "$f" ]] && continue
      [[ -e "$f" ]] || continue
      is_in_scope_file "$f" || continue
      files+=("$f")
    done < <(collect_changed)
    ;;
  path)
    if [[ ! -e "$target_path" ]]; then
      echo "doc-terms: path not found: $target_path" >&2
      exit 2
    fi
    if ! is_in_scope_file "$target_path"; then
      echo "doc-terms: path out of scope: $target_path" >&2
      exit 2
    fi
    files+=("$target_path")
    ;;
esac

if [[ "${#files[@]}" -eq 0 ]]; then
  echo "doc-terms: no docs in scope"
  exit 0
fi

failures=0
while IFS= read -r line; do
  [[ -z "$line" ]] && continue
  [[ "$line" =~ ^# ]] && continue

  forbidden="${line%%|||*}"
  preferred="${line#*|||}"

  if [[ -z "$forbidden" || -z "$preferred" || "$forbidden" == "$line" ]]; then
    echo "doc-terms: invalid rule line (expected 'forbidden|||preferred'): $line" >&2
    exit 2
  fi

  if rg -n -F "$forbidden" "${files[@]}" >/dev/null 2>&1; then
    while IFS= read -r m; do
      echo "doc-terms: $m (use '$preferred')" >&2
      failures=$((failures + 1))
    done < <(rg -n -F "$forbidden" "${files[@]}")
  fi
done <"$RULES_FILE"

if [[ "$failures" -gt 0 ]]; then
  echo "doc-terms: FAILED ($failures issue(s))" >&2
  exit 1
fi

echo "doc-terms: OK"
