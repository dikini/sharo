#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
}

@test "fast feedback includes opt-in live openai smoke gate" {
  run rg 'SHARO_ENABLE_LIVE_OPENAI_SMOKE' "$ROOT/scripts/check-fast-feedback.sh"
  [ "$status" -eq 0 ]

  run rg 'scripts/openai-live-smoke\.sh' "$ROOT/scripts/check-fast-feedback.sh"
  [ "$status" -eq 0 ]
}

@test "manual ci workflow exists for live openai smoke" {
  run rg '^name: openai-live-smoke$' "$ROOT/.github/workflows/openai-live-smoke.yml"
  [ "$status" -eq 0 ]

  run rg 'workflow_dispatch:' "$ROOT/.github/workflows/openai-live-smoke.yml"
  [ "$status" -eq 0 ]
}
