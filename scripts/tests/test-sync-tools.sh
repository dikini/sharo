#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

scripts/check-sync-manifest.sh --path docs/sync/sync-manifest.template.json
scripts/check-sync-manifest.sh --path docs/sync/examples/valid.manifest.json

set +e
scripts/check-sync-manifest.sh --path scripts/tests/sync/invalid.missing-sync-id.manifest.json
invalid_code=$?
set -e
if [[ "$invalid_code" -eq 0 ]]; then
  echo "sync-tools-tests: expected invalid manifest check to fail" >&2
  exit 1
fi

scripts/sync-check.sh --dry-run --manifest docs/sync/examples/valid.manifest.json

set +e
scripts/sync-check.sh --dry-run --manifest docs/sync/examples/valid.manifest.json --include-push-back
push_back_code=$?
set -e
if [[ "$push_back_code" -eq 0 ]]; then
  echo "sync-tools-tests: expected push-back direction guard to fail" >&2
  exit 1
fi

echo "sync-tools-tests: OK"
