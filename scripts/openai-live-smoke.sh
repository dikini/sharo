#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

usage() {
  cat <<'USAGE'
Usage:
  scripts/openai-live-smoke.sh [options]

Runs an end-to-end daemon+CLI turn and surfaces model content from trace/artifacts.
Default mode enforces OpenAI-like provider config and required auth env var.

Options:
  --config-path <path>       Daemon TOML config path (default: ~/.config/sharo/daemon.toml)
  --daemon-env-path <path>   Env file to source for auth vars (default: ~/.config/sharo/daemon.env)
  --no-daemon-env            Do not source daemon env file
  --goal <text>              Goal to submit (default: "Say hello in one sentence")
  --session-label <label>    Session label (default: openai-live-smoke)
  --socket-path <path>       Unix socket path (default: random /tmp path)
  --store-path <path>        Store path (default: random /tmp path)
  --allow-non-openai         Allow non-openai providers (for deterministic/local dry smoke)
  --keep-state               Keep socket/store/log files after exit
  --print-raw                Print full trace/artifacts payloads
  -h, --help                 Show help
USAGE
}

trim_quotes() {
  local value="$1"
  value="${value#\"}"
  value="${value%\"}"
  printf '%s' "$value"
}

read_toml_scalar() {
  local key="$1"
  local path="$2"
  local raw
  raw="$(awk -F= -v k="$key" '
    function strip_inline_comment(s,    i, ch, in_quotes, out) {
      in_quotes = 0
      out = ""
      for (i = 1; i <= length(s); i++) {
        ch = substr(s, i, 1)
        if (ch == "\"") {
          in_quotes = !in_quotes
          out = out ch
          continue
        }
        if (ch == "#" && !in_quotes) {
          break
        }
        out = out ch
      }
      return out
    }
    /^[[:space:]]*#/ { next }
    /^[[:space:]]*\[/ { next }
    {
      left=$1
      gsub(/[[:space:]]/, "", left)
      if (left == k) {
        $1=""
        sub(/^[[:space:]]*=[[:space:]]*/, "")
        print strip_inline_comment($0)
        exit
      }
    }
  ' "$path")"
  if [[ -z "$raw" ]]; then
    printf ''
  else
    raw="$(printf '%s' "$raw" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
    trim_quotes "$raw"
  fi
}

run_daemon_start() {
  if [[ -n "${SHARO_DAEMON_BIN:-}" ]]; then
    "$SHARO_DAEMON_BIN" start "$@"
  else
    cargo run -q -p sharo-daemon -- start "$@"
  fi
}

run_cli() {
  local socket_path="$1"
  shift
  if [[ -n "${SHARO_CLI_BIN:-}" ]]; then
    "$SHARO_CLI_BIN" --transport ipc --socket-path "$socket_path" "$@"
  else
    cargo run -q -p sharo-cli -- --transport ipc --socket-path "$socket_path" "$@"
  fi
}

extract_field() {
  local field="$1"
  local text="$2"
  printf '%s\n' "$text" | tr ' ' '\n' | awk -F= -v f="$field" '$1 == f { print $2; exit }'
}

config_path="${HOME}/.config/sharo/daemon.toml"
daemon_env_path="${HOME}/.config/sharo/daemon.env"
use_daemon_env=true
goal="Say hello in one sentence"
session_label="openai-live-smoke"
socket_path="$(mktemp -u /tmp/sharo-openai-live-XXXXXX.sock)"
store_path="$(mktemp -u /tmp/sharo-openai-live-store-XXXXXX.json)"
allow_non_openai=false
keep_state=false
print_raw=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --config-path)
      shift
      [[ $# -gt 0 ]] || {
        echo "openai-live-smoke: --config-path requires a value" >&2
        exit 2
      }
      config_path="$1"
      ;;
    --goal)
      shift
      [[ $# -gt 0 ]] || {
        echo "openai-live-smoke: --goal requires a value" >&2
        exit 2
      }
      goal="$1"
      ;;
    --daemon-env-path)
      shift
      [[ $# -gt 0 ]] || {
        echo "openai-live-smoke: --daemon-env-path requires a value" >&2
        exit 2
      }
      daemon_env_path="$1"
      ;;
    --no-daemon-env)
      use_daemon_env=false
      ;;
    --session-label)
      shift
      [[ $# -gt 0 ]] || {
        echo "openai-live-smoke: --session-label requires a value" >&2
        exit 2
      }
      session_label="$1"
      ;;
    --socket-path)
      shift
      [[ $# -gt 0 ]] || {
        echo "openai-live-smoke: --socket-path requires a value" >&2
        exit 2
      }
      socket_path="$1"
      ;;
    --store-path)
      shift
      [[ $# -gt 0 ]] || {
        echo "openai-live-smoke: --store-path requires a value" >&2
        exit 2
      }
      store_path="$1"
      ;;
    --allow-non-openai)
      allow_non_openai=true
      ;;
    --keep-state)
      keep_state=true
      ;;
    --print-raw)
      print_raw=true
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "openai-live-smoke: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
  shift
done

if [[ "$use_daemon_env" == true ]]; then
  # shellcheck source=/dev/null
  source "$ROOT/scripts/load-daemon-env.sh"
  load_daemon_env "$daemon_env_path"
fi

if [[ ! -f "$config_path" ]]; then
  echo "openai-live-smoke: config file not found: $config_path" >&2
  exit 1
fi

provider="$(read_toml_scalar "provider" "$config_path")"
auth_env_key="$(read_toml_scalar "auth_env_key" "$config_path")"

if [[ "$allow_non_openai" != true ]]; then
  case "$provider" in
    openai | openai_compatible | openrouter | kimi | glm) ;;
    *)
      echo "openai-live-smoke: provider must be openai-compatible for live smoke (provider=$provider). Use --allow-non-openai to override." >&2
      exit 1
      ;;
  esac

  if [[ -z "$auth_env_key" ]]; then
    echo "openai-live-smoke: config is missing required model.auth_env_key for provider=$provider" >&2
    exit 1
  fi

  if [[ ! "$auth_env_key" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]]; then
    echo "openai-live-smoke: invalid model.auth_env_key value: $auth_env_key" >&2
    exit 1
  fi

  if [[ -z "${!auth_env_key:-}" ]]; then
    echo "openai-live-smoke: missing required auth env var: $auth_env_key" >&2
    exit 1
  fi
fi

daemon_log="$(mktemp /tmp/sharo-openai-live-daemon-log-XXXXXX.txt)"
daemon_pid=""
preserve_daemon_log=false

cleanup() {
  if [[ -n "$daemon_pid" ]]; then
    kill "$daemon_pid" >/dev/null 2>&1 || true
    wait "$daemon_pid" >/dev/null 2>&1 || true
  fi
  if [[ "$keep_state" != true ]]; then
    rm -f "$socket_path" "$store_path"
    if [[ "$preserve_daemon_log" != true ]]; then
      rm -f "$daemon_log"
    fi
  fi
}
trap cleanup EXIT

run_daemon_start --socket-path "$socket_path" --store-path "$store_path" --config-path "$config_path" >"$daemon_log" 2>&1 &
daemon_pid="$!"

session_out=""
for _ in $(seq 1 120); do
  if session_out="$(run_cli "$socket_path" session open --label "$session_label" 2>/dev/null)"; then
    break
  fi
  sleep 0.05
done

if [[ -z "$session_out" ]]; then
  preserve_daemon_log=true
  echo "openai-live-smoke: daemon did not become ready" >&2
  echo "openai-live-smoke: daemon_log=$daemon_log" >&2
  exit 1
fi

session_id="${session_out#session_id=}"

submit_out="$(run_cli "$socket_path" task submit --session-id "$session_id" --goal "$goal")"
task_id="$(extract_field "task_id" "$submit_out")"
if [[ -z "$task_id" ]]; then
  echo "openai-live-smoke: submit output missing task_id" >&2
  echo "$submit_out" >&2
  exit 1
fi

task_out="$(run_cli "$socket_path" task get --task-id "$task_id")"
task_state="$(extract_field "task_state" "$task_out")"

trace_out="$(run_cli "$socket_path" trace get --task-id "$task_id")"
artifacts_out="$(run_cli "$socket_path" artifacts list --task-id "$task_id")"

trace_model_content="$(printf '%s\n' "$trace_out" | sed -n 's/^.*event_kind=model_output_received details=\(.*\)$/\1/p' | head -n 1)"
artifact_model_content="$(printf '%s\n' "$artifacts_out" | sed -n 's/^.*artifact_kind=model_output summary=\(.*\) produced_by_step_id=.*$/\1/p' | head -n 1)"

if [[ -z "$trace_model_content" || -z "$artifact_model_content" ]]; then
  echo "openai-live-smoke: failed to extract model content from trace/artifacts" >&2
  if [[ "$print_raw" == true ]]; then
    echo "TRACE_RAW_BEGIN" >&2
    echo "$trace_out" >&2
    echo "TRACE_RAW_END" >&2
    echo "ARTIFACTS_RAW_BEGIN" >&2
    echo "$artifacts_out" >&2
    echo "ARTIFACTS_RAW_END" >&2
  fi
  exit 1
fi

echo "task_id=$task_id"
echo "task_state=$task_state"
echo "model_content_trace=$trace_model_content"
echo "model_content_artifact=$artifact_model_content"

if [[ "$print_raw" == true ]]; then
  echo "TRACE_RAW_BEGIN"
  echo "$trace_out"
  echo "TRACE_RAW_END"
  echo "ARTIFACTS_RAW_BEGIN"
  echo "$artifacts_out"
  echo "ARTIFACTS_RAW_END"
fi

if [[ "$task_state" != "succeeded" ]]; then
  echo "openai-live-smoke: task did not succeed (task_state=$task_state)" >&2
  exit 1
fi
