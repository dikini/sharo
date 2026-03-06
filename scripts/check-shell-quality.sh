#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

mode="changed"
warn_missing=false

usage() {
  cat <<'USAGE'
Usage:
  scripts/check-shell-quality.sh --changed
  scripts/check-shell-quality.sh --all
  scripts/check-shell-quality.sh --changed --warn-missing
  scripts/check-shell-quality.sh --all --warn-missing
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --changed)
      mode="changed"
      shift
      ;;
    --all)
      mode="all"
      shift
      ;;
    --warn-missing)
      warn_missing=true
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "shell-quality: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

need_tool() {
  local tool="$1"
  local hint="$2"
  if command -v "$tool" >/dev/null 2>&1; then
    return 0
  fi
  if [[ "$warn_missing" == true ]]; then
    echo "shell-quality: warning: missing required tool '$tool'; skipping"
    return 1
  fi
  echo "shell-quality: missing required tool '$tool'" >&2
  echo "shell-quality: install hint: $hint" >&2
  exit 1
}

if ! need_tool shellcheck "apt install -y shellcheck"; then
  exit 0
fi
if ! need_tool shfmt "apt install -y shfmt"; then
  exit 0
fi

collect_files() {
  if [[ "$mode" == "all" ]]; then
    git ls-files '*.sh' '.githooks/*'
    return
  fi

  {
    git diff --name-only
    git diff --cached --name-only
    git ls-files --others --exclude-standard
  } | sed '/^$/d' | sort -u
}

mapfile -t shell_files < <(
  collect_files |
    rg '^(\.githooks/|.*\.sh$)' |
    while IFS= read -r file; do
      if [[ -f "$file" ]]; then
        printf '%s\n' "$file"
      fi
    done
)

if [[ "${#shell_files[@]}" -eq 0 ]]; then
  echo "shell-quality: no shell files in scope"
  exit 0
fi

shfmt -d -i 2 -ci "${shell_files[@]}"
shellcheck -x "${shell_files[@]}"

echo "shell-quality: OK"
