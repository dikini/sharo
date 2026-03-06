#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
}

@test "bootstrap script requires explicit mode" {
  run "$ROOT/scripts/bootstrap-dev.sh"
  [ "$status" -ne 0 ]
  [[ "$output" == *"explicit mode required"* ]]
}

@test "bootstrap script documents check and apply modes" {
  run rg '^  scripts/bootstrap-dev\.sh --check' "$ROOT/scripts/bootstrap-dev.sh"
  [ "$status" -eq 0 ]
  run rg '^  scripts/bootstrap-dev\.sh --apply' "$ROOT/scripts/bootstrap-dev.sh"
  [ "$status" -eq 0 ]
}

@test "just setup target invokes bootstrap apply flow" {
  run rg '^setup:\s*$' "$ROOT/justfile"
  [ "$status" -eq 0 ]
  run rg 'scripts/bootstrap-dev\.sh --apply' "$ROOT/justfile"
  [ "$status" -eq 0 ]
}

@test "bootstrap apply mode runs full verification gate by default" {
  run rg 'scripts/check-fast-feedback\.sh --all' "$ROOT/scripts/bootstrap-dev.sh"
  [ "$status" -eq 0 ]
}

@test "bootstrap checks shell and workflow lint prerequisites" {
  run rg 'ensure_system_tool shellcheck' "$ROOT/scripts/bootstrap-dev.sh"
  [ "$status" -eq 0 ]

  run rg 'ensure_system_tool shfmt' "$ROOT/scripts/bootstrap-dev.sh"
  [ "$status" -eq 0 ]

  run rg 'ensure_system_tool actionlint' "$ROOT/scripts/bootstrap-dev.sh"
  [ "$status" -eq 0 ]
}
