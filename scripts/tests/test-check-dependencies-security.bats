#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
}

@test "dependency security script enforces strict mode and tool checks" {
  run rg '^strict_mode=true$' "$ROOT/scripts/check-dependencies-security.sh"
  [ "$status" -eq 0 ]

  run rg 'cargo deny --version' "$ROOT/scripts/check-dependencies-security.sh"
  [ "$status" -eq 0 ]

  run rg 'cargo audit --version' "$ROOT/scripts/check-dependencies-security.sh"
  [ "$status" -eq 0 ]
}

@test "policy checks workflow invokes dependency security script" {
  run rg 'run: scripts/check-dependencies-security\.sh' "$ROOT/.github/workflows/policy-checks.yml"
  [ "$status" -eq 0 ]
}

