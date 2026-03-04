#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

git config core.hooksPath .githooks
chmod +x .githooks/*
chmod +x scripts/check-*.sh

echo "hooks installed: core.hooksPath=.githooks"
