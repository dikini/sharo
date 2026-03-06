#!/usr/bin/env bash
set -euo pipefail

default_daemon_env_path="${HOME}/.config/sharo/daemon.env"

load_daemon_env() {
  local env_path="${1:-$default_daemon_env_path}"

  if [[ ! -e "$env_path" ]]; then
    return 0
  fi

  if [[ ! -f "$env_path" ]]; then
    echo "daemon-env: path exists but is not a file: $env_path" >&2
    return 1
  fi

  if [[ ! -r "$env_path" ]]; then
    echo "daemon-env: file is not readable: $env_path" >&2
    return 1
  fi

  set -a
  # shellcheck disable=SC1090
  source "$env_path"
  set +a
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  env_path="${1:-$default_daemon_env_path}"
  load_daemon_env "$env_path"
  echo "daemon-env: loaded $env_path"
  echo "daemon-env: note: source this script to keep vars in current shell"
fi
