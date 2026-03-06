#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
}

@test "workflow lint script requires actionlint" {
  run rg 'command -v actionlint' "$ROOT/scripts/check-workflows.sh"
  [ "$status" -eq 0 ]
}

@test "policy checks workflow invokes workflow lint script" {
  run rg 'run: scripts/check-workflows\.sh' "$ROOT/.github/workflows/policy-checks.yml"
  [ "$status" -eq 0 ]
}
