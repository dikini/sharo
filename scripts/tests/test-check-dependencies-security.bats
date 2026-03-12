#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
}

@test "dependency security script enforces strict mode and tool checks" {
  run rg '^strict_mode=true$' "$ROOT/scripts/check-dependencies-security.sh"
  [ "$status" -eq 0 ]

  run rg '^range=""$' "$ROOT/scripts/check-dependencies-security.sh"
  [ "$status" -eq 0 ]

  run rg -- '--range <git-range>' "$ROOT/scripts/check-dependencies-security.sh"
  [ "$status" -eq 0 ]

  run rg 'skipping \(no Cargo inputs changed in range\)' "$ROOT/scripts/check-dependencies-security.sh"
  [ "$status" -eq 0 ]

  run rg 'cargo deny --version' "$ROOT/scripts/check-dependencies-security.sh"
  [ "$status" -eq 0 ]

  run rg 'cargo audit --version' "$ROOT/scripts/check-dependencies-security.sh"
  [ "$status" -eq 0 ]
}

@test "policy checks workflow invokes dependency security script" {
  run rg 'run: scripts/check-dependencies-security\.sh --range "\$\{\{ steps\.range\.outputs\.range \}\}"' "$ROOT/.github/workflows/policy-checks.yml"
  [ "$status" -eq 0 ]
}

@test "dependency security script gates execution to Cargo-impacting ranges" {
  run rg 'git diff --name-only "\$range" \| rg -n '\''\(\^Cargo\\\.lock\$\\|\(\^\|/\)Cargo\\\.toml\$\)'\''' "$ROOT/scripts/check-dependencies-security.sh"
  [ "$status" -eq 0 ]
}

@test "policy checks installs dependency tools only for cargo-impacting ranges" {
  run rg 'cargo_inputs_changed=true' "$ROOT/.github/workflows/policy-checks.yml"
  [ "$status" -eq 0 ]

  run rg 'if: steps\.scope\.outputs\.cargo_inputs_changed == '\''true'\''' "$ROOT/.github/workflows/policy-checks.yml"
  [ "$status" -eq 0 ]
}
