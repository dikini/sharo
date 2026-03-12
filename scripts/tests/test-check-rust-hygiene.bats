#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
}

@test "rust hygiene script supports strict and advisory modes" {
  run rg -- '--strict' "$ROOT/scripts/check-rust-hygiene.sh"
  [ "$status" -eq 0 ]

  run rg -- '--advisory' "$ROOT/scripts/check-rust-hygiene.sh"
  [ "$status" -eq 0 ]
}

@test "rust hygiene script includes udeps msrv and semver checks" {
  run rg 'cargo \+nightly udeps' "$ROOT/scripts/check-rust-hygiene.sh"
  [ "$status" -eq 0 ]

  run rg -- '--rust-version "\$workspace_msrv" -- cargo check --workspace --all-targets' "$ROOT/scripts/check-rust-hygiene.sh"
  [ "$status" -eq 0 ]

  run rg 'cargo msrv verify --workspace' "$ROOT/scripts/check-rust-hygiene.sh"
  [ "$status" -ne 0 ]

  run rg 'cargo semver-checks check-release' "$ROOT/scripts/check-rust-hygiene.sh"
  [ "$status" -eq 0 ]
}

@test "justfile wires rust hygiene command" {
  run rg '^rust-hygiene:\s*$' "$ROOT/justfile"
  [ "$status" -eq 0 ]

  run rg 'scripts/check-rust-hygiene\.sh --advisory --check all' "$ROOT/justfile"
  [ "$status" -eq 0 ]
}

@test "rust hygiene workflow runs strict mode" {
  run rg 'scripts/check-rust-hygiene\.sh --strict --check all --baseline-ref origin/main' "$ROOT/.github/workflows/rust-hygiene.yml"
  [ "$status" -eq 0 ]

  run rg 'uses: taiki-e/install-action@v2' "$ROOT/.github/workflows/rust-hygiene.yml"
  [ "$status" -eq 0 ]

  run rg 'tool: cargo-udeps@0\.1\.60,cargo-msrv@0\.19\.2,cargo-semver-checks@0\.47\.0' "$ROOT/.github/workflows/rust-hygiene.yml"
  [ "$status" -eq 0 ]

  run rg 'cargo install --locked cargo-udeps cargo-msrv cargo-semver-checks' "$ROOT/.github/workflows/rust-hygiene.yml"
  [ "$status" -ne 0 ]
}
