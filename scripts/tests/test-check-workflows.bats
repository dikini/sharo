#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
}

@test "workflow lint script requires actionlint" {
  run rg 'command -v actionlint' "$ROOT/scripts/check-workflows.sh"
  [ "$status" -eq 0 ]
}

@test "policy checks workflow invokes rhysd actionlint action" {
  run rg 'uses: rhysd/actionlint@v1\.7\.11' "$ROOT/.github/workflows/policy-checks.yml"
  [ "$status" -eq 0 ]
}
