#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

mode="all"
target_path=""
range=""

usage() {
  cat <<'USAGE'
Usage:
  scripts/check-doc-portability.sh
  scripts/check-doc-portability.sh --changed
  scripts/check-doc-portability.sh --range <git-range>
  scripts/check-doc-portability.sh --path <file>
USAGE
}

is_in_scope_file() {
  local file_path="$1"
  [[ "$file_path" == README.md || "$file_path" == AGENTS.md || "$file_path" == docs/*.md || "$file_path" == docs/*/*.md || "$file_path" == docs/*/*/*.md || "$file_path" == docs/*/*/*/*.md ]]
}

collect_all() {
  local out=()
  while IFS= read -r file_path; do
    out+=("$file_path")
  done < <(find docs -type f -name '*.md' | sort)
  if [[ -f README.md ]]; then
    out+=("README.md")
  fi
  if [[ -f AGENTS.md ]]; then
    out+=("AGENTS.md")
  fi
  printf '%s\n' "${out[@]}"
}

collect_changed() {
  {
    git diff --name-only -- docs AGENTS.md
    git diff --name-only -- README.md
    git diff --cached --name-only -- docs AGENTS.md
    git diff --cached --name-only -- README.md
    git ls-files --others --exclude-standard -- docs AGENTS.md
    git ls-files --others --exclude-standard -- README.md
  } | sed '/^$/d' | sort -u
}

collect_range() {
  git diff --name-only "$range" -- README.md docs AGENTS.md | sed '/^$/d' | sort -u
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
      if [[ $# -eq 0 ]]; then
        echo "doc-portability: --range requires a value" >&2
        exit 2
      fi
      range="$1"
      shift
      ;;
    --path)
      mode="path"
      shift
      if [[ $# -eq 0 ]]; then
        echo "doc-portability: --path requires a value" >&2
        exit 2
      fi
      target_path="$1"
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "doc-portability: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

files=()
case "$mode" in
  all)
    while IFS= read -r file_path; do
      [[ -n "$file_path" ]] || continue
      files+=("$file_path")
    done < <(collect_all)
    ;;
  changed)
    while IFS= read -r file_path; do
      [[ -n "$file_path" ]] || continue
      [[ -e "$file_path" ]] || continue
      is_in_scope_file "$file_path" || continue
      files+=("$file_path")
    done < <(collect_changed)
    ;;
  range)
    while IFS= read -r file_path; do
      [[ -n "$file_path" ]] || continue
      [[ -e "$file_path" ]] || continue
      is_in_scope_file "$file_path" || continue
      files+=("$file_path")
    done < <(collect_range)
    ;;
  path)
    if [[ ! -e "$target_path" ]]; then
      echo "doc-portability: path not found: $target_path" >&2
      exit 2
    fi
    if ! is_in_scope_file "$target_path"; then
      echo "doc-portability: path out of scope: $target_path" >&2
      exit 2
    fi
    files+=("$target_path")
    ;;
esac

if [[ "${#files[@]}" -eq 0 ]]; then
  echo "doc-portability: no docs in scope"
  exit 0
fi

failures=0
while IFS= read -r match; do
  file_path="${match%%:*}"
  rest="${match#*:}"
  link="$(echo "$rest" | sed -E 's/.*\[[^]]+\]\(([^)]+)\).*/\1/')"
  link="${link%% \"*}"
  target="${link%%#*}"

  case "$link" in
    http://* | https://* | mailto:* | \#*)
      continue
      ;;
  esac

  [[ -n "$target" ]] || continue

  if [[ "$target" =~ (^/home/|^/Users/|^[A-Za-z]:\\Users\\) ]]; then
    echo "doc-portability: machine-local path: $file_path -> $link" >&2
    failures=$((failures + 1))
  fi

  if [[ "$target" == *"/.worktrees/"* || "$target" == *".worktrees/"* ]]; then
    echo "doc-portability: worktree-local path: $file_path -> $link" >&2
    failures=$((failures + 1))
  fi
done < <(rg -H -n -o '\[[^]]+\]\(([^)]+)\)' "${files[@]}")

if [[ "$failures" -gt 0 ]]; then
  echo "doc-portability: FAILED ($failures issue(s))" >&2
  exit 1
fi

echo "doc-portability: OK"
