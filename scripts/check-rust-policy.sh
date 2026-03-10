#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  scripts/check-rust-policy.sh
  scripts/check-rust-policy.sh --path <workspace-or-package-cargo-toml>
USAGE
}

target_manifest="Cargo.toml"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --path)
      shift
      if [[ $# -eq 0 ]]; then
        echo "rust-policy: --path requires a value" >&2
        exit 2
      fi
      target_manifest="$1"
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "rust-policy: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

if [[ ! -f "$target_manifest" ]]; then
  echo "rust-policy: Cargo.toml not present, skipping"
  exit 0
fi

version_ge() {
  local current="$1"
  local required="$2"
  local c_major c_minor c_patch r_major r_minor r_patch

  IFS=. read -r c_major c_minor c_patch <<<"$current"
  IFS=. read -r r_major r_minor r_patch <<<"$required"

  c_major="${c_major:-0}"
  c_minor="${c_minor:-0}"
  c_patch="${c_patch:-0}"
  r_major="${r_major:-0}"
  r_minor="${r_minor:-0}"
  r_patch="${r_patch:-0}"

  if ((c_major != r_major)); then
    ((c_major > r_major))
    return
  fi
  if ((c_minor != r_minor)); then
    ((c_minor > r_minor))
    return
  fi
  ((c_patch >= r_patch))
}

check_manifest() {
  local manifest="$1"
  local edition rust_version

  if [[ ! -f "$manifest" ]]; then
    echo "rust-policy check failed: missing manifest $manifest" >&2
    return 1
  fi

  edition="$(sed -nE 's/^[[:space:]]*edition[[:space:]]*=[[:space:]]*"([^"]+)".*$/\1/p' "$manifest" | head -n1)"
  rust_version="$(sed -nE 's/^[[:space:]]*rust-version[[:space:]]*=[[:space:]]*"([^"]+)".*$/\1/p' "$manifest" | head -n1)"

  if [[ "$edition" != "2024" ]]; then
    echo "rust-policy check failed: $manifest edition must be \"2024\" (found: ${edition:-<missing>})" >&2
    return 1
  fi

  if [[ -z "$rust_version" ]]; then
    echo "rust-policy check failed: $manifest must set rust-version >= 1.93" >&2
    return 1
  fi

  if ! version_ge "$rust_version" "1.93.0"; then
    echo "rust-policy check failed: $manifest rust-version must be >= 1.93 (found: $rust_version)" >&2
    return 1
  fi

  return 0
}

collect_workspace_members() {
  local root_manifest="$1"
  awk '
    /^\[workspace\]/ { in_ws=1; next }
    /^\[/ && !/^\[workspace\]/ { if (in_ws) exit }
    in_ws && /members[[:space:]]*=/ { in_members=1 }
    in_members {
      while (match($0, /"[^"]+"/)) {
        m = substr($0, RSTART + 1, RLENGTH - 2)
        print m
        $0 = substr($0, RSTART + RLENGTH)
      }
      if ($0 ~ /\]/) {
        in_members=0
      }
    }
  ' "$root_manifest"
}

root_dir="$(cd "$(dirname "$target_manifest")" && pwd)"
root_manifest="$root_dir/$(basename "$target_manifest")"

if rg -n '^\[workspace\]' "$root_manifest" >/dev/null 2>&1; then
  mapfile -t members < <(collect_workspace_members "$root_manifest")

  if [[ "${#members[@]}" -eq 0 ]]; then
    echo "rust-policy check failed: workspace has no members in $root_manifest" >&2
    exit 1
  fi

  failures=0
  checked=0
  for member in "${members[@]}"; do
    manifest="$root_dir/$member/Cargo.toml"
    if check_manifest "$manifest"; then
      checked=$((checked + 1))
    else
      failures=$((failures + 1))
    fi
  done

  if [[ "$failures" -gt 0 ]]; then
    echo "rust-policy: FAILED ($failures member manifest issue(s))" >&2
    exit 1
  fi

  echo "rust-policy: OK (workspace members checked=$checked)"
  exit 0
fi

if check_manifest "$root_manifest"; then
  echo "rust-policy: OK (edition=2024, rust-version set)"
  exit 0
fi

exit 1
