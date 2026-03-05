#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

TEST_FILE="scripts/tests/test-mvp-matrix-map.bats"

usage() {
  cat <<'USAGE'
Usage:
  scripts/check-mvp-matrix-map.sh

Runs the dedicated MVP verification matrix mapping quality gate via Bats.
USAGE
}

if [[ "${1-}" == "-h" || "${1-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ $# -gt 0 ]]; then
  echo "mvp-matrix-map: unknown argument '$1'" >&2
  usage
  exit 2
fi

if [[ ! -f "$TEST_FILE" ]]; then
  echo "mvp-matrix-map: missing gate test file: $TEST_FILE" >&2
  echo "mvp-matrix-map: add the Bats test to keep this gate executable." >&2
  exit 1
fi

bats_bin="$(scripts/install-bats.sh)"

echo "mvp-matrix-map: running quality gate via $TEST_FILE"
if "$bats_bin" "$TEST_FILE"; then
  echo "mvp-matrix-map: OK"
  exit 0
fi

echo "mvp-matrix-map: FAIL (see Bats output above)" >&2
exit 1
