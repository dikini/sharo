#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
}

@test "workflow lint script requires actionlint" {
  run rg '\.tools/actionlint/actionlint' "$ROOT/scripts/check-workflows.sh"
  [ "$status" -eq 0 ]

  run rg 'command -v actionlint' "$ROOT/scripts/check-workflows.sh"
  [ "$status" -eq 0 ]

  run rg "missing required tool 'actionlint'" "$ROOT/scripts/check-workflows.sh"
  [ "$status" -eq 0 ]
}

@test "policy checks workflow no longer runs dedicated actionlint step" {
  run rg 'actionlint' "$ROOT/.github/workflows/policy-checks.yml"
  [ "$status" -ne 0 ]
}
