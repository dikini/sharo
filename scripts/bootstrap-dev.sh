#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

mode="check"
run_verify=true

usage() {
  cat <<'USAGE'
Usage:
  scripts/bootstrap-dev.sh --check [--no-verify]
  scripts/bootstrap-dev.sh --apply [--no-verify]

Options:
  --check      Validate required toolchain/tools without installing them.
  --apply      Install or configure missing project dependencies.
  --no-verify  Skip final scripts/check-fast-feedback.sh --all run.
USAGE
}

require_mode=false
while [[ $# -gt 0 ]]; do
  case "$1" in
    --check)
      mode="check"
      require_mode=true
      shift
      ;;
    --apply)
      mode="apply"
      require_mode=true
      shift
      ;;
    --no-verify)
      run_verify=false
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "bootstrap-dev: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

if [[ "$require_mode" != true ]]; then
  echo "bootstrap-dev: explicit mode required (--check or --apply)" >&2
  usage
  exit 2
fi

need_command() {
  local command_name="$1"
  local install_hint="$2"
  if command -v "$command_name" >/dev/null 2>&1; then
    return 0
  fi
  echo "bootstrap-dev: missing command '$command_name' ($install_hint)" >&2
  return 1
}

ensure_toolchain() {
  if ! need_command rustup "install rustup first"; then
    exit 1
  fi
  if ! need_command cargo "install Rust toolchain first"; then
    exit 1
  fi

  local channel
  channel="$(awk -F'"' '/^channel = / {print $2}' rust-toolchain.toml)"
  if [[ -z "$channel" ]]; then
    echo "bootstrap-dev: could not parse channel from rust-toolchain.toml" >&2
    exit 1
  fi

  if rustup toolchain list | rg -q "^${channel}"; then
    echo "bootstrap-dev: rust toolchain present ($channel)"
  elif [[ "$mode" == "apply" ]]; then
    echo "bootstrap-dev: installing rust toolchain ($channel)"
    rustup toolchain install "$channel" --profile minimal --component clippy --component rustfmt
  else
    echo "bootstrap-dev: missing rust toolchain ($channel)" >&2
    exit 1
  fi
}

ensure_bats() {
  local bats_bin
  bats_bin="$(find .tools/bats -type f -name bats -perm -u+x 2>/dev/null | head -n1 || true)"
  if [[ -n "$bats_bin" ]]; then
    echo "bootstrap-dev: bats present ($bats_bin)"
    return 0
  fi

  if [[ "$mode" == "apply" ]]; then
    echo "bootstrap-dev: installing bats"
    bats_bin="$(scripts/install-bats.sh)"
    echo "bootstrap-dev: bats installed ($bats_bin)"
    return 0
  fi

  echo "bootstrap-dev: bats missing (run scripts/install-bats.sh or --apply)" >&2
  return 1
}

ensure_just() {
  if command -v just >/dev/null 2>&1; then
    echo "bootstrap-dev: just present ($(command -v just))"
    return 0
  fi

  if [[ "$mode" == "apply" ]]; then
    echo "bootstrap-dev: installing just"
    cargo install --locked just
    return 0
  fi

  echo "bootstrap-dev: just missing (use --apply to install)" >&2
  return 1
}

ensure_cargo_tool() {
  local subcommand="$1"
  local package="$2"
  if cargo "$subcommand" --version >/dev/null 2>&1; then
    echo "bootstrap-dev: cargo $subcommand present"
    return 0
  fi

  if [[ "$mode" == "apply" ]]; then
    echo "bootstrap-dev: installing $package"
    cargo install --locked "$package"
    return 0
  fi

  echo "bootstrap-dev: cargo $subcommand missing (use --apply to install $package)" >&2
  return 1
}

ensure_hooks() {
  local configured
  configured="$(git config --get core.hooksPath || true)"
  if [[ "$configured" == ".githooks" ]]; then
    echo "bootstrap-dev: git hooks configured (.githooks)"
    return 0
  fi

  if [[ "$mode" == "apply" ]]; then
    echo "bootstrap-dev: installing git hooks"
    scripts/install-hooks.sh
    return 0
  fi

  echo "bootstrap-dev: git hooks not configured (expected core.hooksPath=.githooks)" >&2
  return 1
}

ensure_toolchain
ensure_bats
ensure_just
ensure_cargo_tool deny cargo-deny
ensure_cargo_tool audit cargo-audit
ensure_cargo_tool nextest cargo-nextest
ensure_hooks

if [[ "$run_verify" == true ]]; then
  echo "bootstrap-dev: running full verification gate"
  scripts/check-fast-feedback.sh --all
fi

echo "bootstrap-dev: OK"
