#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

mode="changed"
range=""

usage() {
  cat <<'USAGE'
Usage:
  scripts/run-shell-tests.sh --changed
  scripts/run-shell-tests.sh --range <git-range>
  scripts/run-shell-tests.sh --all
USAGE
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
      [[ $# -gt 0 ]] || { echo "shell-tests: --range requires a value" >&2; exit 2; }
      range="$1"
      shift
      ;;
    --all)
      mode="all"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "shell-tests: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

collect_changed_files() {
  case "$mode" in
    all)
      find scripts/tests -type f -name '*.bats' 2>/dev/null || true
      ;;
    range)
      git diff --name-only "$range"
      ;;
    changed)
      {
        git diff --name-only
        git diff --cached --name-only
        git ls-files --others --exclude-standard
      } | sort -u
      ;;
  esac
}

mapfile -t changed < <(collect_changed_files | sed '/^$/d' | sort -u)

if [[ "$mode" == "all" ]]; then
  mapfile -t test_files < <(find scripts/tests -type f -name '*.bats' | sort)
else
  mapfile -t changed_bats < <(printf '%s\n' "${changed[@]:-}" | rg '^scripts/tests/.+\.bats$' || true)
  if [[ "${#changed_bats[@]}" -gt 0 ]]; then
    test_files=("${changed_bats[@]}")
  elif printf '%s\n' "${changed[@]:-}" | rg -n '^(scripts/(check-|sync-check\.sh|install-bats\.sh|run-shell-tests\.sh|lib/)|docs/sync/|docs/tasks/)' >/dev/null 2>&1; then
    mapfile -t test_files < <(find scripts/tests -type f -name '*.bats' | sort)
  else
    echo "shell-tests: no shell-test-impacting files changed, skipping"
    exit 0
  fi
fi

if [[ "${#test_files[@]}" -eq 0 ]]; then
  echo "shell-tests: no bats test files found"
  exit 0
fi

bats_bin="$(scripts/install-bats.sh)"
echo "shell-tests: using bats at $bats_bin"
echo "shell-tests: running ${#test_files[@]} file(s)"
"$bats_bin" "${test_files[@]}"
