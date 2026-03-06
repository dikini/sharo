#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

cargo test -p sharo-daemon --test scenario_a \
  duplicate_submit_during_inflight_reasoning_does_not_double_execute_provider \
  -- --nocapture

cargo test -p sharo-daemon --test scenario_a \
  same_process_retry_after_terminal_save_failure_is_not_stuck_in_progress \
  -- --nocapture

cargo test -p sharo-daemon --test daemon_ipc \
  status_requests_remain_responsive_under_parallel_slow_submits \
  -- --nocapture

cargo test -p sharo-daemon --test daemon_ipc \
  ctrl_c_waits_for_inflight_request_completion \
  -- --nocapture
