#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

mode="all"
target_path=""
target_range=""

usage() {
  cat <<'EOF'
Usage:
  scripts/check-sync-manifest.sh
  scripts/check-sync-manifest.sh --path <manifest.json>
  scripts/check-sync-manifest.sh --changed
  scripts/check-sync-manifest.sh --range <git-range>
EOF
}

is_manifest_file() {
  local f="$1"
  [[ "$f" == docs/sync/*.json || "$f" == docs/sync/*/*.json || "$f" == docs/sync/*/*/*.json || "$f" == scripts/tests/sync/*.json ]]
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --path)
      mode="path"
      shift
      if [[ $# -eq 0 ]]; then
        echo "sync-manifest-check: --path requires a value" >&2
        exit 2
      fi
      target_path="$1"
      shift
      ;;
    --changed)
      mode="changed"
      shift
      ;;
    --range)
      mode="range"
      shift
      if [[ $# -eq 0 ]]; then
        echo "sync-manifest-check: --range requires a value" >&2
        exit 2
      fi
      target_range="$1"
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "sync-manifest-check: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

collect_all() {
  find docs/sync -type f -name '*.json' 2>/dev/null | sort
}

collect_changed() {
  {
    git diff --name-only -- docs/sync
    git diff --cached --name-only -- docs/sync
    git ls-files --others --exclude-standard -- docs/sync
  } | sed '/^$/d' | sort -u
}

collect_range() {
  git diff --name-only "$target_range" -- docs/sync | sed '/^$/d' | sort -u
}

files=()
case "$mode" in
  all)
    while IFS= read -r f; do
      [[ -z "$f" ]] && continue
      files+=("$f")
    done < <(collect_all)
    ;;
  path)
    if [[ ! -f "$target_path" ]]; then
      echo "sync-manifest-check: file not found: $target_path" >&2
      exit 2
    fi
    if ! is_manifest_file "$target_path"; then
      echo "sync-manifest-check: path out of scope: $target_path" >&2
      exit 2
    fi
    files+=("$target_path")
    ;;
  changed)
    while IFS= read -r f; do
      [[ -z "$f" ]] && continue
      [[ -f "$f" ]] || continue
      is_manifest_file "$f" || continue
      files+=("$f")
    done < <(collect_changed)
    ;;
  range)
    while IFS= read -r f; do
      [[ -z "$f" ]] && continue
      [[ -f "$f" ]] || continue
      is_manifest_file "$f" || continue
      files+=("$f")
    done < <(collect_range)
    ;;
esac

if [[ "${#files[@]}" -eq 0 ]]; then
  echo "sync-manifest-check: no manifest json files in scope"
  exit 0
fi

failures=0
fail() {
  echo "sync-manifest-check: $1" >&2
  failures=$((failures + 1))
}

required_top_keys=(
  "sync_id"
  "request_ref"
  "direction"
  "intent"
  "created_at_utc"
  "updated_at_utc"
  "items"
)

required_item_keys=(
  "item_id"
  "direction"
  "source_ref"
  "staged_path"
  "canonical_path"
  "hash_sha256"
  "status"
  "timestamp_utc"
)

is_allowed_direction() {
  [[ "$1" == "vault->repo" || "$1" == "repo->vault" ]]
}

is_allowed_status() {
  case "$1" in
    pulled | validated | promoted | pushed_back | failed) return 0 ;;
    *) return 1 ;;
  esac
}

for f in "${files[@]}"; do
  for k in "${required_top_keys[@]}"; do
    if ! rg -n "\"${k}\"[[:space:]]*:" "$f" >/dev/null 2>&1; then
      fail "$f missing top-level key '$k'"
    fi
  done

  direction="$(sed -nE 's/^[[:space:]]*"direction"[[:space:]]*:[[:space:]]*"([^"]+)".*$/\1/p' "$f" | head -n1)"
  if [[ -z "$direction" ]]; then
    fail "$f missing top-level direction value"
  elif ! is_allowed_direction "$direction"; then
    fail "$f has invalid top-level direction '$direction'"
  fi

  item_count="$(rg -o '"item_id"[[:space:]]*:' "$f" | wc -l | tr -d ' ')"
  if [[ "$item_count" -lt 1 ]]; then
    fail "$f must contain at least one item with 'item_id'"
  fi

  for k in "${required_item_keys[@]}"; do
    count="$(rg -o "\"${k}\"[[:space:]]*:" "$f" | wc -l | tr -d ' ')"
    if [[ "$count" -lt "$item_count" ]]; then
      fail "$f missing required item key '$k' for one or more items"
    fi
  done

  while IFS= read -r s; do
    [[ -z "$s" ]] && continue
    if ! is_allowed_status "$s"; then
      fail "$f has invalid item status '$s'"
    fi
  done < <(rg -o '"status"[[:space:]]*:[[:space:]]*"[^"]+"' "$f" | sed -E 's/.*"status"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/')

  while IFS= read -r d; do
    [[ -z "$d" ]] && continue
    if ! is_allowed_direction "$d"; then
      fail "$f has invalid item direction '$d'"
    fi
  done < <(rg -o '"direction"[[:space:]]*:[[:space:]]*"[^"]+"' "$f" | sed -E 's/.*"direction"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/')
done

if [[ "$failures" -gt 0 ]]; then
  echo "sync-manifest-check: FAILED (${failures} issue(s))" >&2
  exit 1
fi

echo "sync-manifest-check: OK (${#files[@]} file(s))"
