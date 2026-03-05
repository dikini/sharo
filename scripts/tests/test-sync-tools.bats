#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
  cd "$ROOT"
}

@test "sync manifest template validates" {
  run scripts/check-sync-manifest.sh --path docs/sync/sync-manifest.template.json
  [ "$status" -eq 0 ]
}

@test "valid example manifest validates" {
  run scripts/check-sync-manifest.sh --path docs/sync/examples/valid.manifest.json
  [ "$status" -eq 0 ]
}

@test "invalid fixture fails validation" {
  run scripts/check-sync-manifest.sh --path scripts/tests/sync/invalid.missing-sync-id.manifest.json
  [ "$status" -ne 0 ]
}

@test "sync-check dry-run succeeds for valid manifest" {
  run scripts/sync-check.sh --dry-run --manifest docs/sync/examples/valid.manifest.json
  [ "$status" -eq 0 ]
}

@test "push-back guard fails for vault->repo manifest" {
  run scripts/sync-check.sh --dry-run --manifest docs/sync/examples/valid.manifest.json --include-push-back
  [ "$status" -ne 0 ]
}
