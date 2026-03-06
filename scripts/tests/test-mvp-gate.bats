#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
  cd "$ROOT"
}

@test "mvp_readiness_checklist_has_no_open_required_items" {
  run rg '^- \[ \] ' docs/specs/mvp.md
  [ "$status" -ne 0 ]
}

@test "task_registry_states_consistent_with_mvp_gate" {
  run bash -lc "for id in TASK-MVP-SLICE-000 TASK-MVP-SLICE-001 TASK-MVP-SLICE-002 TASK-MVP-SLICE-003 TASK-MVP-SLICE-004 TASK-MVP-SLICE-005; do rg -q \"^\${id},[^,]+,[^,]+,[^,]+,done,\" docs/tasks/tasks.csv || exit 1; done"
  [ "$status" -eq 0 ]
}

@test "full_policy_and_test_gate_passes_on_mvp_state" {
  run bash -lc "scripts/check-rust-policy.sh && scripts/check-tasks-registry.sh && scripts/check-tasks-sync.sh --changed"
  [ "$status" -eq 0 ]
}
