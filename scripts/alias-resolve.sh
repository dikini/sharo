#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
REGISTRY="$ROOT/docs/aliases.toml"

usage() {
  cat <<'USAGE'
Usage:
  scripts/alias-resolve.sh --list
  scripts/alias-resolve.sh <alias>

Examples:
  scripts/alias-resolve.sh @spec:mvp
  scripts/alias-resolve.sh plan:vault-sync-protocol
USAGE
}

if [[ ! -f "$REGISTRY" ]]; then
  echo "alias-resolve: registry not found: $REGISTRY" >&2
  exit 1
fi

list_aliases() {
  awk '
    /^\[aliases\]$/ { in_aliases=1; next }
    /^\[/ { if (in_aliases) exit }
    in_aliases && $0 ~ /^[[:space:]]*"[^"]+"[[:space:]]*=/ {
      line=$0
      sub(/^[[:space:]]*"/, "", line)
      sub(/"[[:space:]]*=.*/, "", line)
      print line
    }
  ' "$REGISTRY" | sort
}

resolve_alias() {
  local key="$1"
  awk -v k="$key" '
    /^\[aliases\]$/ { in_aliases=1; next }
    /^\[/ { if (in_aliases) exit }
    in_aliases && $0 ~ /^[[:space:]]*"[^"]+"[[:space:]]*=/ {
      line=$0
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", line)
      sub(/^"/, "", line)
      split(line, parts, "=")
      lhs=parts[1]
      rhs=substr(line, index(line, "=")+1)
      gsub(/[[:space:]]+$/, "", lhs)
      gsub(/^"|"$/, "", lhs)
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", rhs)
      gsub(/^"|"$/, "", rhs)
      if (lhs == k) {
        print rhs
        exit 0
      }
    }
  ' "$REGISTRY"
}

if [[ $# -ne 1 ]]; then
  usage
  exit 2
fi

case "$1" in
  -h|--help)
    usage
    exit 0
    ;;
  --list)
    list_aliases
    exit 0
    ;;
  *)
    alias_key="$1"
    alias_key="${alias_key#@}"
    ;;
esac

target_rel="$(resolve_alias "$alias_key")"
if [[ -z "$target_rel" ]]; then
  echo "alias-resolve: unknown alias '$alias_key'" >&2
  exit 1
fi

# Registry must stay repo-relative and never escape the repository root.
if [[ "$target_rel" == /* || "$target_rel" == *".."* ]]; then
  echo "alias-resolve: invalid non-relative target for alias '$alias_key': $target_rel" >&2
  exit 1
fi

target_abs="$ROOT/$target_rel"
if [[ ! -e "$target_abs" ]]; then
  echo "alias-resolve: target for alias '$alias_key' does not exist: $target_abs" >&2
  exit 1
fi

printf '%s\n' "$target_abs"
