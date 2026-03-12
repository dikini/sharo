#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

mode="changed"
range=""
iterations=3

usage() {
  cat <<'USAGE'
Usage:
  scripts/check-flaky-regressions.sh --changed
  scripts/check-flaky-regressions.sh --range <git-range>
  scripts/check-flaky-regressions.sh --all
  scripts/check-flaky-regressions.sh --iterations <count>
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --changed)
      mode="changed"
      shift
      ;;
    --range)
      mode="range"
      shift
      if [[ $# -eq 0 ]]; then
        echo "flaky-regressions: --range requires a value" >&2
        exit 2
      fi
      range="$1"
      shift
      ;;
    --all)
      mode="all"
      shift
      ;;
    --iterations)
      shift
      if [[ $# -eq 0 ]]; then
        echo "flaky-regressions: --iterations requires a value" >&2
        exit 2
      fi
      iterations="$1"
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "flaky-regressions: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

if ! [[ "$iterations" =~ ^[1-9][0-9]*$ ]]; then
  echo "flaky-regressions: iterations must be a positive integer" >&2
  exit 2
fi

if [[ ! -f "Cargo.toml" ]]; then
  echo "flaky-regressions: Cargo.toml not present, skipping"
  exit 0
fi

changed_files() {
  case "$mode" in
    all)
      find crates -type f \( -name '*.rs' -o -name 'Cargo.toml' \) 2>/dev/null || true
      ;;
    range)
      git diff --name-only "$range"
      ;;
    changed)
      {
        git diff --name-only
        git diff --cached --name-only
        git ls-files --others --exclude-standard
      } | sort -u
      ;;
  esac
}

files="$(changed_files | sed '/^$/d' || true)"

if [[ "$mode" != "all" ]]; then
  if ! echo "$files" | rg -n '^(Cargo\.toml|Cargo\.lock|rust-toolchain\.toml|crates/sharo-daemon/(src|tests)/.+|crates/sharo-core/src/.+)$' >/dev/null 2>&1; then
    echo "flaky-regressions: no daemon-impacting files changed, skipping"
    exit 0
  fi
fi

run_case() {
  local test_file="$1"
  local test_name="$2"
  cargo test -p sharo-daemon --test "$test_file" "$test_name" -- --nocapture
}

for iteration in $(seq 1 "$iterations"); do
  echo "flaky-regressions: iteration $iteration/$iterations"
  run_case scenario_a duplicate_submit_during_inflight_reasoning_does_not_double_execute_provider
  run_case scenario_a same_process_retry_after_terminal_save_failure_is_not_stuck_in_progress
  run_case daemon_ipc status_requests_remain_responsive_under_parallel_slow_submits
  run_case daemon_ipc ctrl_c_waits_for_inflight_request_completion
done

echo "flaky-regressions: OK"
