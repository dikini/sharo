#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
}

@test "fast feedback includes opt-in fuzz smoke gate" {
  run rg 'SHARO_ENABLE_FUZZ_SMOKE' "$ROOT/scripts/check-fast-feedback.sh"
  [ "$status" -eq 0 ]

  run rg 'scripts/check-fuzz\.sh --smoke --changed' "$ROOT/scripts/check-fast-feedback.sh"
  [ "$status" -eq 0 ]
}

@test "policy checks workflow enforces fuzz smoke gate" {
  run rg 'Install cargo-fuzz' "$ROOT/.github/workflows/policy-checks.yml"
  [ "$status" -eq 0 ]

  run rg 'scripts/check-fuzz\.sh --smoke --all' "$ROOT/.github/workflows/policy-checks.yml"
  [ "$status" -eq 0 ]
}

@test "check-fuzz script declares required modes" {
  run rg -- '--smoke --changed' "$ROOT/scripts/check-fuzz.sh"
  [ "$status" -eq 0 ]

  run rg -- '--full --all' "$ROOT/scripts/check-fuzz.sh"
  [ "$status" -eq 0 ]
}
