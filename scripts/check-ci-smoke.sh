#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

scripts/doc-lint.sh --changed --strict-new
scripts/check-doc-terms.sh --changed
scripts/check-doc-portability.sh --changed
scripts/check-shell-quality.sh --changed --warn-missing
scripts/check-tasks-registry.sh
scripts/check-tasks-sync.sh --changed
scripts/check-conflict-determinism.sh
scripts/check-rust-policy.sh
scripts/check-sync-manifest.sh --changed
scripts/check-mvp-matrix-map.sh
scripts/check-knot-diff.sh --mapping docs/tasks/knot-diff-mapping.csv
scripts/check-research-references.sh --registry docs/tasks/research-reference-rules.csv

echo "ci-smoke: OK"
