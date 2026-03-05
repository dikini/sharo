#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
  cd "$ROOT"
  tmpdir="$(mktemp -d)"
}

teardown() {
  rm -rf "$tmpdir"
}

@test "rust-policy passes for valid workspace manifests" {
  pass_case="$tmpdir/pass"
  mkdir -p "$pass_case/crates/a" "$pass_case/crates/b"
  cat > "$pass_case/Cargo.toml" <<'TOML'
[workspace]
members = ["crates/a", "crates/b"]
resolver = "2"
TOML
  cat > "$pass_case/crates/a/Cargo.toml" <<'TOML'
[package]
name = "a"
version = "0.1.0"
edition = "2024"
rust-version = "1.93"
TOML
  cat > "$pass_case/crates/b/Cargo.toml" <<'TOML'
[package]
name = "b"
version = "0.1.0"
edition = "2024"
rust-version = "1.94"
TOML

  run scripts/check-rust-policy.sh --path "$pass_case/Cargo.toml"
  [ "$status" -eq 0 ]
}

@test "rust-policy rejects bad edition" {
  fail_edition="$tmpdir/fail-edition"
  mkdir -p "$fail_edition/crates/a"
  cat > "$fail_edition/Cargo.toml" <<'TOML'
[workspace]
members = ["crates/a"]
resolver = "2"
TOML
  cat > "$fail_edition/crates/a/Cargo.toml" <<'TOML'
[package]
name = "a"
version = "0.1.0"
edition = "2021"
rust-version = "1.93"
TOML

  run scripts/check-rust-policy.sh --path "$fail_edition/Cargo.toml"
  [ "$status" -ne 0 ]
}

@test "rust-policy rejects too-low rust-version" {
  fail_version="$tmpdir/fail-version"
  mkdir -p "$fail_version/crates/a"
  cat > "$fail_version/Cargo.toml" <<'TOML'
[workspace]
members = ["crates/a"]
resolver = "2"
TOML
  cat > "$fail_version/crates/a/Cargo.toml" <<'TOML'
[package]
name = "a"
version = "0.1.0"
edition = "2024"
rust-version = "1.92"
TOML

  run scripts/check-rust-policy.sh --path "$fail_version/Cargo.toml"
  [ "$status" -ne 0 ]
}
