#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
}

@test "ci smoke script keeps docs and policy gates but skips duplicate Rust test lanes" {
  run rg '^scripts/check-doc-lint\.sh$' "$ROOT/scripts/check-ci-smoke.sh"
  [ "$status" -ne 0 ]

  run rg '^scripts/doc-lint\.sh --changed --strict-new$' "$ROOT/scripts/check-ci-smoke.sh"
  [ "$status" -eq 0 ]

  run rg '^scripts/check-doc-terms\.sh --changed$' "$ROOT/scripts/check-ci-smoke.sh"
  [ "$status" -eq 0 ]

  run rg '^scripts/check-doc-portability\.sh --changed$' "$ROOT/scripts/check-ci-smoke.sh"
  [ "$status" -eq 0 ]

  run rg '^scripts/check-shell-quality\.sh --changed --warn-missing$' "$ROOT/scripts/check-ci-smoke.sh"
  [ "$status" -eq 0 ]

  run rg '^scripts/check-rust-policy\.sh$' "$ROOT/scripts/check-ci-smoke.sh"
  [ "$status" -eq 0 ]

  run rg '^scripts/check-rust-tests\.sh' "$ROOT/scripts/check-ci-smoke.sh"
  [ "$status" -ne 0 ]

  run rg '^scripts/check-daemon-invariants\.sh$' "$ROOT/scripts/check-ci-smoke.sh"
  [ "$status" -ne 0 ]

  run rg '^scripts/check-durability-signals\.sh$' "$ROOT/scripts/check-ci-smoke.sh"
  [ "$status" -ne 0 ]
}
