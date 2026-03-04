#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

git config core.hooksPath .githooks
chmod +x .githooks/*
chmod +x scripts/check-*.sh
if [[ -f scripts/sync-check.sh ]]; then
  chmod +x scripts/sync-check.sh
fi
if [[ -d scripts/tests ]]; then
  find scripts/tests -type f -name '*.sh' -exec chmod +x {} +
fi

echo "hooks installed: core.hooksPath=.githooks"
