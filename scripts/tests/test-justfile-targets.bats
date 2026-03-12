#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
}

@test "justfile includes required workflow targets" {
  run rg '^init-repo:\s*$' "$ROOT/justfile"
  [ "$status" -eq 0 ]

  run rg '^extract-backbone:\s*$' "$ROOT/justfile"
  [ "$status" -eq 0 ]

  run rg '^init-backbone-repo dest project=' "$ROOT/justfile"
  [ "$status" -eq 0 ]

  run rg '^verify:\s*$' "$ROOT/justfile"
  [ "$status" -eq 0 ]

  run rg '^verify-ci:\s*$' "$ROOT/justfile"
  [ "$status" -eq 0 ]

  run rg '^fast-feedback:\s*$' "$ROOT/justfile"
  [ "$status" -eq 0 ]

  run rg '^prepush-policy:\s*$' "$ROOT/justfile"
  [ "$status" -eq 0 ]

  run rg '^merge-gate:\s*$' "$ROOT/justfile"
  [ "$status" -eq 0 ]

  run rg '^daemon-invariants:\s*$' "$ROOT/justfile"
  [ "$status" -eq 0 ]

  run rg '^flaky-regressions:\s*$' "$ROOT/justfile"
  [ "$status" -eq 0 ]

  run rg '^shell-quality:\s*$' "$ROOT/justfile"
  [ "$status" -eq 0 ]

  run rg '^doc-portability:\s*$' "$ROOT/justfile"
  [ "$status" -eq 0 ]

  run rg '^workflow-lint:\s*$' "$ROOT/justfile"
  [ "$status" -eq 0 ]

  run rg '^rust-hygiene:\s*$' "$ROOT/justfile"
  [ "$status" -eq 0 ]
}

@test "policy checks workflow uses just verify-ci entrypoint" {
  run rg 'run: just verify-ci' "$ROOT/.github/workflows/policy-checks.yml"
  [ "$status" -eq 0 ]
}

@test "policy checks workflow keeps dedicated range-based policy follow-ups" {
  run rg 'Resolve commit range' "$ROOT/.github/workflows/policy-checks.yml"
  [ "$status" -eq 0 ]

  run rg 'scripts/check-dependencies-security\.sh --range "\$\{\{ steps\.range\.outputs\.range \}\}"' "$ROOT/.github/workflows/policy-checks.yml"
  [ "$status" -eq 0 ]

  run rg 'scripts/run-shell-tests\.sh --range "\$\{\{ steps\.range\.outputs\.range \}\}"' "$ROOT/.github/workflows/policy-checks.yml"
  [ "$status" -eq 0 ]
}

@test "policy checks avoids duplicate property and loom coverage and sets CI cache-friendly env" {
  run rg 'CARGO_INCREMENTAL: "0"' "$ROOT/.github/workflows/policy-checks.yml"
  [ "$status" -eq 0 ]

  run rg 'Run property test profile' "$ROOT/.github/workflows/policy-checks.yml"
  [ "$status" -ne 0 ]

  run rg 'Run loom model checks' "$ROOT/.github/workflows/policy-checks.yml"
  [ "$status" -ne 0 ]
}
