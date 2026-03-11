#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

cargo test -p sharo-daemon --bin sharo-daemon \
  store::tests::post_rename_directory_sync_failure_emits_warning_signal \
  -- --exact --nocapture
