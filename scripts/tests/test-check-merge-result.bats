#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
}

@test "merge-result script runs clippy and all fast-feedback checks" {
  run rg '^scripts/check-fast-feedback\.sh --all$' "$ROOT/scripts/check-merge-result.sh"
  [ "$status" -eq 0 ]

  run rg '^cargo clippy --all-targets --all-features -- -D warnings$' "$ROOT/scripts/check-merge-result.sh"
  [ "$status" -eq 0 ]
}

@test "merge-result workflow includes merge queue trigger and script entrypoint" {
  run rg '^  merge_group:$' "$ROOT/.github/workflows/merge-result-gate.yml"
  [ "$status" -eq 0 ]

  run rg 'run: scripts/check-merge-result\.sh' "$ROOT/.github/workflows/merge-result-gate.yml"
  [ "$status" -eq 0 ]
}

