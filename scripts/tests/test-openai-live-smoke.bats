#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
  cd "$ROOT"
  TMP_DIR="$(mktemp -d)"
  cargo build -q -p sharo-cli -p sharo-daemon
  export SHARO_CLI_BIN="$ROOT/target/debug/sharo"
  export SHARO_DAEMON_BIN="$ROOT/target/debug/sharo-daemon"
}

teardown() {
  rm -rf "$TMP_DIR"
}

@test "openai_live_smoke_help_succeeds" {
  run scripts/openai-live-smoke.sh --help
  [ "$status" -eq 0 ]
  [[ "$output" == *"Usage:"* ]]
}

@test "openai_live_smoke_requires_auth_env_when_openai" {
  config="$TMP_DIR/daemon.toml"
  cat > "$config" <<'CFG'
[model]
provider = "openai"
base_url = "https://api.openai.com"
model_id = "gpt-4.1-mini"
auth_env_key = "SHARO_TEST_MISSING_OPENAI_KEY"
timeout_ms = 1000
CFG

  run scripts/openai-live-smoke.sh --config-path "$config"
  [ "$status" -ne 0 ]
  [[ "$output" == *"missing required auth env var"* ]]
}

@test "openai_live_smoke_accepts_inline_comment_after_auth_env_key" {
  config="$TMP_DIR/daemon.toml"
  cat > "$config" <<'CFG'
[model]
provider = "openai"
base_url = "https://api.openai.com"
model_id = "gpt-4.1-mini"
auth_env_key = "SHARO_TEST_MISSING_OPENAI_KEY" # inline comment
timeout_ms = 1000
CFG

  run scripts/openai-live-smoke.sh --config-path "$config"
  [ "$status" -ne 0 ]
  [[ "$output" == *"missing required auth env var"* ]]
  [[ "$output" != *"invalid variable name"* ]]
}

@test "openai_live_smoke_keeps_daemon_log_when_readiness_fails" {
  fake_daemon="$TMP_DIR/fake-daemon.sh"
  cat > "$fake_daemon" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail
echo "fake-daemon-start $*" >&2
sleep 30
FAKE
  chmod +x "$fake_daemon"

  config="$TMP_DIR/deterministic.toml"
  cat > "$config" <<'CFG'
[model]
provider = "deterministic"
model_id = "mock"
timeout_ms = 1000
CFG

  SHARO_DAEMON_BIN="$fake_daemon" run scripts/openai-live-smoke.sh --allow-non-openai --config-path "$config"
  [ "$status" -ne 0 ]
  [[ "$output" == *"daemon did not become ready"* ]]
  [[ "$output" == *"daemon_log="* ]]
  daemon_log_path="$(printf '%s\n' "$output" | sed -n 's/^.*daemon_log=\(.*\)$/\1/p' | tail -n 1)"
  [ -n "$daemon_log_path" ]
  [ -f "$daemon_log_path" ]
  run grep -q "fake-daemon-start" "$daemon_log_path"
  [ "$status" -eq 0 ]
}

@test "openai_live_smoke_parses_task_id_from_submit_output" {
  config="$TMP_DIR/deterministic.toml"
  cat > "$config" <<'CFG'
[model]
provider = "deterministic"
model_id = "mock"
timeout_ms = 1000
CFG

  run scripts/openai-live-smoke.sh --allow-non-openai --config-path "$config" --goal "hello from bats"
  [ "$status" -eq 0 ]
  [[ "$output" == *"task_id=task-"* ]]
}

@test "openai_live_smoke_deterministic_mode_surfaces_model_content" {
  config="$TMP_DIR/deterministic.toml"
  cat > "$config" <<'CFG'
[model]
provider = "deterministic"
model_id = "mock"
timeout_ms = 1000
CFG

  run scripts/openai-live-smoke.sh --allow-non-openai --config-path "$config" --goal "hello from bats"
  [ "$status" -eq 0 ]
  [[ "$output" == *"model_content_trace=deterministic-response"* ]]
  [[ "$output" == *"model_content_artifact=deterministic-response"* ]]
}
