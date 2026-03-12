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

@test "policy checks workflow no longer owns nightly fuzz installation" {
  run rg 'Install cargo-fuzz' "$ROOT/.github/workflows/policy-checks.yml"
  [ "$status" -ne 0 ]

  run rg 'Install nightly toolchain for fuzzing' "$ROOT/.github/workflows/policy-checks.yml"
  [ "$status" -ne 0 ]

  run rg 'scripts/check-fuzz\.sh' "$ROOT/.github/workflows/policy-checks.yml"
  [ "$status" -ne 0 ]
}

@test "nightly fuzz workflow installs nightly toolchain and cargo-fuzz" {
  run rg '^name: nightly-fuzz$' "$ROOT/.github/workflows/nightly-fuzz.yml"
  [ "$status" -eq 0 ]

  run rg '^  schedule:$' "$ROOT/.github/workflows/nightly-fuzz.yml"
  [ "$status" -eq 0 ]

  run rg '^  workflow_dispatch:$' "$ROOT/.github/workflows/nightly-fuzz.yml"
  [ "$status" -eq 0 ]

  run rg 'Install cargo-fuzz' "$ROOT/.github/workflows/nightly-fuzz.yml"
  [ "$status" -eq 0 ]

  run rg 'Install nightly toolchain for fuzzing' "$ROOT/.github/workflows/nightly-fuzz.yml"
  [ "$status" -eq 0 ]

  run rg 'scripts/check-fuzz\.sh --smoke --all' "$ROOT/.github/workflows/nightly-fuzz.yml"
  [ "$status" -eq 0 ]
}

@test "check-fuzz script declares required modes" {
  run rg -- '--smoke --changed' "$ROOT/scripts/check-fuzz.sh"
  [ "$status" -eq 0 ]

  run rg -- '--full --all' "$ROOT/scripts/check-fuzz.sh"
  [ "$status" -eq 0 ]

  run rg 'SHARO_FUZZ_SEED' "$ROOT/scripts/check-fuzz.sh"
  [ "$status" -eq 0 ]

  run rg -- '-seed=1' "$ROOT/scripts/check-fuzz.sh"
  [ "$status" -ne 0 ]
}
