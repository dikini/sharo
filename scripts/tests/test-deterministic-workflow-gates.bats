#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
}

@test "merge-compat script delegates to merge-result gate" {
  run rg '^scripts/check-merge-result\.sh$' "$ROOT/scripts/check-merge-compat.sh"
  [ "$status" -eq 0 ]
}

@test "conflict determinism script enforces marker rejection and allowlist" {
  run rg "rg -n '\^\(<<<<<<<\|=======\|>>>>>>>\)'" "$ROOT/scripts/check-conflict-determinism.sh"
  [ "$status" -eq 0 ]

  run rg "^allowlist_regex='\\^\\(CHANGELOG\\\\\\.md\\|docs/tasks/tasks\\\\\\.csv\\|Cargo\\\\\\.lock\\)\\$'$|^allowlist_regex='\\^\\(CHANGELOG\\.md\\|docs/tasks/tasks\\.csv\\|Cargo\\.lock\\)\\$'$|^allowlist_regex='\\^\\(CHANGELOG\\\\.md\\|docs/tasks/tasks\\\\.csv\\|Cargo\\\\.lock\\)\\$'$" "$ROOT/scripts/check-conflict-determinism.sh"
  [ "$status" -eq 0 ]
}

@test "daemon invariant gate includes required high-risk cases" {
  run rg 'duplicate_submit_during_inflight_reasoning_does_not_double_execute_provider' "$ROOT/scripts/check-daemon-invariants.sh"
  [ "$status" -eq 0 ]

  run rg 'same_process_retry_after_terminal_save_failure_is_not_stuck_in_progress' "$ROOT/scripts/check-daemon-invariants.sh"
  [ "$status" -eq 0 ]

  run rg 'status_requests_remain_responsive_under_parallel_slow_submits' "$ROOT/scripts/check-daemon-invariants.sh"
  [ "$status" -eq 0 ]

  run rg 'ctrl_c_waits_for_inflight_request_completion' "$ROOT/scripts/check-daemon-invariants.sh"
  [ "$status" -eq 0 ]
}

@test "durability signal gate checks explicit warning assertion coverage" {
  run rg 'post_rename_directory_sync_failure_emits_warning_signal' "$ROOT/scripts/check-durability-signals.sh"
  [ "$status" -eq 0 ]
}

@test "fast feedback includes deterministic workflow hardening gates" {
  run rg '^scripts/check-workflows\.sh "\$\{workflow_lint_args\[@\]\}"$' "$ROOT/scripts/check-fast-feedback.sh"
  [ "$status" -eq 0 ]

  run rg '\[\[ "\$\{CI:-false\}" == "true" \]\]' "$ROOT/scripts/check-fast-feedback.sh"
  [ "$status" -eq 0 ]

  run rg '^scripts/check-conflict-determinism\.sh$' "$ROOT/scripts/check-fast-feedback.sh"
  [ "$status" -eq 0 ]

  run rg '^scripts/check-daemon-invariants\.sh$' "$ROOT/scripts/check-fast-feedback.sh"
  [ "$status" -eq 0 ]

  run rg '^scripts/check-durability-signals\.sh$' "$ROOT/scripts/check-fast-feedback.sh"
  [ "$status" -eq 0 ]
}
