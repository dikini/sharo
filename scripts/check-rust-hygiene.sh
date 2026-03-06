#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

strict_mode=false
check_target="all"
baseline_ref="origin/main"

usage() {
  cat <<'USAGE'
Usage:
  scripts/check-rust-hygiene.sh --advisory|--strict [--check all|udeps|msrv|semver] [--baseline-ref <git-ref>]
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --advisory)
      strict_mode=false
      shift
      ;;
    --strict)
      strict_mode=true
      shift
      ;;
    --check)
      shift
      [[ $# -gt 0 ]] || {
        echo "rust-hygiene: --check requires a value" >&2
        exit 2
      }
      check_target="$1"
      shift
      ;;
    --baseline-ref)
      shift
      [[ $# -gt 0 ]] || {
        echo "rust-hygiene: --baseline-ref requires a value" >&2
        exit 2
      }
      baseline_ref="$1"
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "rust-hygiene: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

warn_or_fail() {
  local message="$1"
  if [[ "$strict_mode" == true ]]; then
    echo "rust-hygiene: $message" >&2
    exit 1
  fi
  echo "rust-hygiene: warning: $message"
}

run_with_mode() {
  local label="$1"
  shift
  echo "rust-hygiene: running $label"
  if "$@"; then
    return 0
  fi
  warn_or_fail "$label failed"
}

run_udeps() {
  if ! cargo udeps --version >/dev/null 2>&1; then
    warn_or_fail "cargo-udeps missing (cargo install --locked cargo-udeps)"
    return 0
  fi
  run_with_mode "cargo +nightly udeps" cargo +nightly udeps --workspace --all-targets
}

run_msrv() {
  if ! cargo msrv --version >/dev/null 2>&1; then
    warn_or_fail "cargo-msrv missing (cargo install --locked cargo-msrv)"
    return 0
  fi
  run_with_mode "cargo msrv verify" cargo msrv verify --workspace -- cargo check --workspace --all-targets
}

run_semver() {
  if ! cargo semver-checks --version >/dev/null 2>&1; then
    warn_or_fail "cargo-semver-checks missing (cargo install --locked cargo-semver-checks)"
    return 0
  fi

  if ! git rev-parse --verify "$baseline_ref" >/dev/null 2>&1; then
    warn_or_fail "baseline ref '$baseline_ref' not found for semver checks"
    return 0
  fi

  run_with_mode \
    "cargo semver-checks check-release (sharo-core)" \
    cargo semver-checks check-release --manifest-path crates/sharo-core/Cargo.toml --baseline-rev "$baseline_ref"
}

case "$check_target" in
  all)
    run_udeps
    run_msrv
    run_semver
    ;;
  udeps)
    run_udeps
    ;;
  msrv)
    run_msrv
    ;;
  semver)
    run_semver
    ;;
  *)
    echo "rust-hygiene: invalid --check value '$check_target'" >&2
    usage
    exit 2
    ;;
esac

echo "rust-hygiene: OK"
