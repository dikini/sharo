#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
}

@test "shell quality script enforces shellcheck and shfmt presence" {
  run rg 'need_tool shellcheck' "$ROOT/scripts/check-shell-quality.sh"
  [ "$status" -eq 0 ]

  run rg 'need_tool shfmt' "$ROOT/scripts/check-shell-quality.sh"
  [ "$status" -eq 0 ]
}

@test "shell quality script supports changed and all modes" {
  run rg -- '--changed' "$ROOT/scripts/check-shell-quality.sh"
  [ "$status" -eq 0 ]

  run rg -- '--all' "$ROOT/scripts/check-shell-quality.sh"
  [ "$status" -eq 0 ]
}

@test "policy checks workflow invokes shell quality script" {
  run rg 'run: scripts/check-shell-quality\.sh --all' "$ROOT/.github/workflows/policy-checks.yml"
  [ "$status" -eq 0 ]
}

@test "policy checks workflow runs shell tests in range mode" {
  run rg 'run: scripts/run-shell-tests\.sh --range "\$\{\{ steps\.range\.outputs\.range \}\}"' "$ROOT/.github/workflows/policy-checks.yml"
  [ "$status" -eq 0 ]
}
