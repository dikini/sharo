#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

scripts/check-fast-feedback.sh --all
cargo clippy --all-targets --all-features -- -D warnings
