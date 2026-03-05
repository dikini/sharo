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
