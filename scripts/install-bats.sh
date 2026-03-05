#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

BATS_VERSION="1.13.0"
BATS_ARCHIVE_SHA256="a85e12b8828271a152b338ca8109aa23493b57950987c8e6dff97ba492772ff3"
BATS_URL="https://github.com/bats-core/bats-core/archive/refs/tags/v${BATS_VERSION}.tar.gz"
INSTALL_DIR="$ROOT/.tools/bats/${BATS_VERSION}"
BATS_BIN="$INSTALL_DIR/bin/bats"

if [[ -x "$BATS_BIN" ]]; then
  echo "$BATS_BIN"
  exit 0
fi

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT
archive_path="$tmp_dir/bats.tar.gz"

curl -fsSL "$BATS_URL" -o "$archive_path"
actual_sha="$(sha256sum "$archive_path" | awk '{print $1}')"
if [[ "$actual_sha" != "$BATS_ARCHIVE_SHA256" ]]; then
  echo "install-bats: checksum mismatch expected=$BATS_ARCHIVE_SHA256 actual=$actual_sha" >&2
  exit 1
fi

tar -xzf "$archive_path" -C "$tmp_dir"
extracted_dir="$tmp_dir/bats-core-${BATS_VERSION}"
if [[ ! -d "$extracted_dir" ]]; then
  echo "install-bats: extracted directory not found: $extracted_dir" >&2
  exit 1
fi

mkdir -p "$(dirname "$INSTALL_DIR")"
rm -rf "$INSTALL_DIR"
mv "$extracted_dir" "$INSTALL_DIR"

if [[ ! -x "$BATS_BIN" ]]; then
  echo "install-bats: bats executable not found after install: $BATS_BIN" >&2
  exit 1
fi

echo "$BATS_BIN"
