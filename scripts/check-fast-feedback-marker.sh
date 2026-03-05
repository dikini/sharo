#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

git_dir="$(git rev-parse --git-dir)"
marker_file="$git_dir/.fast-feedback.ok"

usage() {
  cat <<'USAGE'
Usage:
  scripts/check-fast-feedback-marker.sh
USAGE
}

if [[ $# -gt 0 ]]; then
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "fast-feedback-marker: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
fi

if [[ ! -f "$marker_file" ]]; then
  echo "fast-feedback-marker: missing marker. Run scripts/check-fast-feedback.sh before commit." >&2
  exit 1
fi

marker_head="$(sed -nE 's/^head=(.*)$/\1/p' "$marker_file" | head -n1)"
marker_sha="$(sed -nE 's/^status_sha=(.*)$/\1/p' "$marker_file" | head -n1)"

if [[ -z "$marker_head" || -z "$marker_sha" ]]; then
  echo "fast-feedback-marker: malformed marker. Re-run scripts/check-fast-feedback.sh." >&2
  exit 1
fi

current_head="$(git rev-parse HEAD)"
current_sha="$(git status --porcelain=v1 --untracked-files=all | sha256sum | awk '{print $1}')"

if [[ "$marker_head" != "$current_head" ]]; then
  echo "fast-feedback-marker: marker HEAD mismatch. Re-run scripts/check-fast-feedback.sh." >&2
  exit 1
fi

if [[ "$marker_sha" != "$current_sha" ]]; then
  echo "fast-feedback-marker: working tree changed since marker. Re-run scripts/check-fast-feedback.sh." >&2
  exit 1
fi

echo "fast-feedback-marker: OK"
