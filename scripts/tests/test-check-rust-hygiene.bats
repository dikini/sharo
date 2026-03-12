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

  run rg 'Install cargo hygiene tools' "$ROOT/.github/workflows/rust-hygiene.yml"
  [ "$status" -eq 0 ]

  run rg 'https://github.com/\$\{repo\}/releases/download/v\$\{version\}/\$\{asset\}' "$ROOT/.github/workflows/rust-hygiene.yml"
  [ "$status" -eq 0 ]

  run rg 'cargo-udeps-v\$\{CARGO_UDEPS_VERSION\}-x86_64-unknown-linux-gnu\.tar\.gz' "$ROOT/.github/workflows/rust-hygiene.yml"
  [ "$status" -eq 0 ]

  run rg 'cargo-msrv-x86_64-unknown-linux-gnu-v\$\{CARGO_MSRV_VERSION\}\.tgz' "$ROOT/.github/workflows/rust-hygiene.yml"
  [ "$status" -eq 0 ]

  run rg 'cargo-semver-checks-x86_64-unknown-linux-gnu\.tar\.gz' "$ROOT/.github/workflows/rust-hygiene.yml"
  [ "$status" -eq 0 ]

  run rg 'taiki-e/install-action@v2' "$ROOT/.github/workflows/rust-hygiene.yml"
  [ "$status" -ne 0 ]

  run rg 'cargo install --locked cargo-udeps cargo-msrv cargo-semver-checks' "$ROOT/.github/workflows/rust-hygiene.yml"
  [ "$status" -ne 0 ]
}
