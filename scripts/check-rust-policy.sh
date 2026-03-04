#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

if [[ ! -f "Cargo.toml" ]]; then
  echo "rust-policy: Cargo.toml not present, skipping"
  exit 0
fi

edition="$(sed -nE 's/^[[:space:]]*edition[[:space:]]*=[[:space:]]*"([^"]+)".*$/\1/p' Cargo.toml | head -n1)"
rust_version="$(sed -nE 's/^[[:space:]]*rust-version[[:space:]]*=[[:space:]]*"([^"]+)".*$/\1/p' Cargo.toml | head -n1)"

if [[ "$edition" != "2024" ]]; then
  echo "rust-policy check failed: Cargo.toml edition must be \"2024\" (found: ${edition:-<missing>})" >&2
  exit 1
fi

if [[ -z "$rust_version" ]]; then
  echo "rust-policy check failed: Cargo.toml must set rust-version >= 1.93" >&2
  exit 1
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

  if (( c_major != r_major )); then
    (( c_major > r_major ))
    return
  fi
  if (( c_minor != r_minor )); then
    (( c_minor > r_minor ))
    return
  fi
  (( c_patch >= r_patch ))
}

if version_ge "$rust_version" "1.93.0"; then
  echo "rust-policy: OK (edition=2024, rust-version=$rust_version)"
  exit 0
fi

echo "rust-policy check failed: rust-version must be >= 1.93 (found: $rust_version)" >&2
exit 1
