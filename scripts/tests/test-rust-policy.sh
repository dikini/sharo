#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

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

scripts/check-rust-policy.sh --path "$pass_case/Cargo.toml"

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

if scripts/check-rust-policy.sh --path "$fail_edition/Cargo.toml"; then
  echo "test-rust-policy: expected edition failure" >&2
  exit 1
fi

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

if scripts/check-rust-policy.sh --path "$fail_version/Cargo.toml"; then
  echo "test-rust-policy: expected rust-version failure" >&2
  exit 1
fi

echo "test-rust-policy: OK"
