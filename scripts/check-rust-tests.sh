#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

mode="changed"
range=""

usage() {
  cat <<'USAGE'
Usage:
  scripts/check-rust-tests.sh
  scripts/check-rust-tests.sh --changed
  scripts/check-rust-tests.sh --range <git-range>
  scripts/check-rust-tests.sh --all
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
      if [[ $# -eq 0 ]]; then
        echo "rust-tests: --range requires a value" >&2
        exit 2
      fi
      range="$1"
      shift
      ;;
    --all)
      mode="all"
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "rust-tests: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

if [[ ! -f "Cargo.toml" ]]; then
  echo "rust-tests: Cargo.toml not present, skipping"
  exit 0
fi

changed_files() {
  case "$mode" in
    all)
      find crates -type f \( -name '*.rs' -o -name 'Cargo.toml' \) 2>/dev/null || true
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

files="$(changed_files | sed '/^$/d' || true)"

# Run tests only when Rust-relevant files changed unless --all is requested.
if [[ "$mode" != "all" ]]; then
  if ! echo "$files" | rg -n '^(Cargo\.toml|Cargo\.lock|rust-toolchain\.toml|crates/.+\.rs|crates/.+/Cargo\.toml|scripts/check-rust-policy\.sh|scripts/tests/test-rust-policy\.(sh|bats))$' >/dev/null 2>&1; then
    echo "rust-tests: no Rust-impacting files changed, skipping"
    exit 0
  fi
fi

echo "rust-tests: running workspace tests via scripts/check-tests.sh"
scripts/check-tests.sh --workspace
