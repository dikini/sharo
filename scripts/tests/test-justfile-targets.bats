#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
}

@test "justfile includes required workflow targets" {
  run rg '^verify:\s*$' "$ROOT/justfile"
  [ "$status" -eq 0 ]

  run rg '^fast-feedback:\s*$' "$ROOT/justfile"
  [ "$status" -eq 0 ]

  run rg '^merge-gate:\s*$' "$ROOT/justfile"
  [ "$status" -eq 0 ]

  run rg '^daemon-invariants:\s*$' "$ROOT/justfile"
  [ "$status" -eq 0 ]
}

@test "policy checks workflow uses just verify entrypoint" {
  run rg 'run: just verify' "$ROOT/.github/workflows/policy-checks.yml"
  [ "$status" -eq 0 ]
}

