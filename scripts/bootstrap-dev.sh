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
    -h | --help)
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

ensure_system_tool() {
  local command_name="$1"
  local install_hint="$2"
  if command -v "$command_name" >/dev/null 2>&1; then
    echo "bootstrap-dev: $command_name present ($(command -v "$command_name"))"
    return 0
  fi

  echo "bootstrap-dev: missing command '$command_name' ($install_hint)" >&2
  return 1
}

ensure_actionlint() {
  local actionlint_version="1.7.11"
  local checksums_file="actionlint_${actionlint_version}_checksums.txt"
  local checksums_sha256="7d588eeb1ceb1e926b5618162a082453e1618b7772597e4ef8270e08777a8114"
  local release_base_url="https://github.com/rhysd/actionlint/releases/download/v${actionlint_version}"
  local metadata_url="https://api.github.com/repos/rhysd/actionlint/releases/tags/v${actionlint_version}"
  local local_actionlint="$ROOT/.tools/actionlint/actionlint"
  local os=""
  local arch=""
  local archive_name=""
  local expected_archive_sha256=""
  local metadata_archive_digest=""
  local tmpdir=""
  local local_version=""
  local path_version=""

  sha256_file() {
    local file_path="$1"
    if command -v sha256sum >/dev/null 2>&1; then
      sha256sum "$file_path" | awk '{print $1}'
      return 0
    fi
    if command -v shasum >/dev/null 2>&1; then
      shasum -a 256 "$file_path" | awk '{print $1}'
      return 0
    fi
    return 1
  }

  sha256_check() {
    local expected="$1"
    local file_path="$2"
    local actual=""
    actual="$(sha256_file "$file_path" || true)"
    if [[ -z "$actual" ]]; then
      return 1
    fi
    [[ "$actual" == "$expected" ]]
  }

  actionlint_version_of() {
    local bin_path="$1"
    "$bin_path" -version 2>/dev/null | head -n1
  }

  if [[ -x "$local_actionlint" ]]; then
    local_version="$(actionlint_version_of "$local_actionlint" || true)"
    if [[ "$local_version" == "$actionlint_version" ]]; then
      echo "bootstrap-dev: actionlint present ($local_actionlint, v$local_version)"
      return 0
    fi
    echo "bootstrap-dev: actionlint local version mismatch (found '${local_version:-unknown}', expected '$actionlint_version')" >&2
    if [[ "$mode" != "apply" ]]; then
      return 1
    fi
    echo "bootstrap-dev: reinstalling pinned actionlint v$actionlint_version"
    rm -f "$local_actionlint"
  fi

  if command -v actionlint >/dev/null 2>&1; then
    path_version="$(actionlint_version_of "$(command -v actionlint)" || true)"
    if [[ "$path_version" == "$actionlint_version" ]]; then
      echo "bootstrap-dev: actionlint present on PATH ($(command -v actionlint), v$path_version)"
      return 0
    fi
    echo "bootstrap-dev: actionlint on PATH has version '${path_version:-unknown}', expected '$actionlint_version'" >&2
    if [[ "$mode" != "apply" ]]; then
      return 1
    fi
    echo "bootstrap-dev: installing pinned actionlint v$actionlint_version into .tools/actionlint"
  fi

  if [[ "$mode" == "apply" ]]; then
    if ! command -v curl >/dev/null 2>&1; then
      echo "bootstrap-dev: missing command 'curl' (required to download actionlint release assets)" >&2
      return 1
    fi
    if ! command -v sha256sum >/dev/null 2>&1 && ! command -v shasum >/dev/null 2>&1; then
      echo "bootstrap-dev: missing SHA-256 tool ('sha256sum' or 'shasum') required for archive integrity verification" >&2
      return 1
    fi
    if ! command -v python3 >/dev/null 2>&1; then
      echo "bootstrap-dev: missing command 'python3' (required to parse release metadata digest)" >&2
      return 1
    fi

    case "${OSTYPE:-}" in
      linux-*)
        os="linux"
        ;;
      darwin*)
        os="darwin"
        ;;
      freebsd*)
        os="freebsd"
        ;;
      *)
        echo "bootstrap-dev: unsupported OS for actionlint auto-install: '${OSTYPE:-unknown}'" >&2
        return 1
        ;;
    esac

    case "$(uname -m)" in
      x86_64)
        arch="amd64"
        ;;
      i?86)
        arch="386"
        ;;
      aarch64 | arm64)
        arch="arm64"
        ;;
      arm*)
        arch="armv6"
        ;;
      *)
        echo "bootstrap-dev: unsupported architecture for actionlint auto-install: '$(uname -m)'" >&2
        return 1
        ;;
    esac

    archive_name="actionlint_${actionlint_version}_${os}_${arch}.tar.gz"

    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' RETURN

    echo "bootstrap-dev: installing actionlint $actionlint_version to .tools/actionlint"
    mkdir -p "$ROOT/.tools/actionlint"

    curl -fsSL "${release_base_url}/${checksums_file}" -o "$tmpdir/$checksums_file"
    if ! sha256_check "$checksums_sha256" "$tmpdir/$checksums_file"; then
      echo "bootstrap-dev: actionlint checksums file verification failed" >&2
      return 1
    fi

    expected_archive_sha256="$(awk -v name="$archive_name" '$2 == name {print $1}' "$tmpdir/$checksums_file")"
    if [[ -z "$expected_archive_sha256" ]]; then
      echo "bootstrap-dev: checksum entry not found for $archive_name in $checksums_file" >&2
      return 1
    fi

    curl -fsSL "$metadata_url" -o "$tmpdir/release-metadata.json"

    metadata_archive_digest="$(
      python3 - "$archive_name" "$tmpdir/release-metadata.json" <<'PY'
import json
import sys

archive_name = sys.argv[1]
payload_path = sys.argv[2]
with open(payload_path, "r", encoding="utf-8") as fh:
    payload = json.load(fh)
for asset in payload.get("assets", []):
    if asset.get("name") == archive_name:
        digest = asset.get("digest", "")
        if digest.startswith("sha256:"):
            print(digest.split(":", 1)[1])
            raise SystemExit(0)
raise SystemExit(1)
PY
    )"
    if [[ -z "$metadata_archive_digest" ]]; then
      echo "bootstrap-dev: missing archive digest in GitHub release metadata for $archive_name" >&2
      return 1
    fi
    if [[ "$metadata_archive_digest" != "$expected_archive_sha256" ]]; then
      echo "bootstrap-dev: release metadata digest mismatch for $archive_name" >&2
      return 1
    fi

    curl -fsSL "${release_base_url}/${archive_name}" -o "$tmpdir/$archive_name"
    if ! sha256_check "$expected_archive_sha256" "$tmpdir/$archive_name"; then
      echo "bootstrap-dev: actionlint archive checksum verification failed for $archive_name" >&2
      return 1
    fi

    tar -xzf "$tmpdir/$archive_name" -C "$ROOT/.tools/actionlint" actionlint
    chmod +x "$local_actionlint"
    local_version="$(actionlint_version_of "$local_actionlint" || true)"
    if [[ "$local_version" != "$actionlint_version" ]]; then
      echo "bootstrap-dev: installed actionlint version mismatch (found '${local_version:-unknown}', expected '$actionlint_version')" >&2
      return 1
    fi
    return 0
  fi

  echo "bootstrap-dev: missing command 'actionlint' (use --apply to install pinned verified binary into .tools/actionlint)" >&2
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
ensure_cargo_tool udeps cargo-udeps
ensure_cargo_tool msrv cargo-msrv
ensure_cargo_tool semver-checks cargo-semver-checks
ensure_system_tool shellcheck "apt install -y shellcheck"
ensure_system_tool shfmt "apt install -y shfmt"
ensure_actionlint
ensure_hooks

if [[ "$run_verify" == true ]]; then
  echo "bootstrap-dev: running full verification gate"
  scripts/check-fast-feedback.sh --all
fi

echo "bootstrap-dev: OK"
