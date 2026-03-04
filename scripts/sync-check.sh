#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

manifest=""
dry_run=false
include_push_back=false

usage() {
  cat <<'EOF'
Usage:
  scripts/sync-check.sh --manifest <manifest.json> [--dry-run] [--include-push-back]
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --manifest)
      shift
      if [[ $# -eq 0 ]]; then
        echo "sync-check: --manifest requires a value" >&2
        exit 2
      fi
      manifest="$1"
      shift
      ;;
    --dry-run)
      dry_run=true
      shift
      ;;
    --include-push-back)
      include_push_back=true
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "sync-check: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

if [[ -z "$manifest" ]]; then
  echo "sync-check: --manifest is required" >&2
  exit 2
fi

scripts/check-sync-manifest.sh --path "$manifest"

direction="$(sed -nE 's/^[[:space:]]*"direction"[[:space:]]*:[[:space:]]*"([^"]+)".*$/\1/p' "$manifest" | head -n1)"
sync_id="$(sed -nE 's/^[[:space:]]*"sync_id"[[:space:]]*:[[:space:]]*"([^"]+)".*$/\1/p' "$manifest" | head -n1)"

if [[ -z "$sync_id" ]]; then
  echo "sync-check: unable to read sync_id from manifest: $manifest" >&2
  exit 1
fi

if [[ "$include_push_back" == "true" && "$direction" != "repo->vault" ]]; then
  echo "sync-check: --include-push-back requires direction=repo->vault (found: $direction)" >&2
  exit 1
fi

stages=("pull" "validate" "promote")
if [[ "$include_push_back" == "true" ]]; then
  stages+=("push-back")
fi

echo "sync-check: sync_id=$sync_id direction=$direction"
echo "sync-check: stage order=${stages[*]}"

if [[ "$dry_run" == "true" ]]; then
  echo "sync-check: dry-run mode enabled; manifest and canonical files are not modified"
  exit 0
fi

tmp="$(mktemp)"
cp "$manifest" "$tmp"

final_status="promoted"
if [[ "$include_push_back" == "true" ]]; then
  final_status="pushed_back"
fi

sed -E "s/(\"status\"[[:space:]]*:[[:space:]]*\")([a-z_]+)(\")/\1${final_status}\3/g" "$tmp" > "${tmp}.next"
mv "${tmp}.next" "$tmp"

now_utc="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
sed -E "0,/\"updated_at_utc\"[[:space:]]*:[[:space:]]*\"[^\"]+\"/s//\"updated_at_utc\": \"${now_utc}\"/" "$tmp" > "${tmp}.next"
mv "${tmp}.next" "$tmp"

mv "$tmp" "$manifest"

echo "sync-check: updated manifest statuses to '$final_status'"
