#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
  TMP_DIR="$(mktemp -d)"
}

teardown() {
  rm -rf "$TMP_DIR"
}

@test "check-tests prefers nextest when available" {
  mkdir -p "$TMP_DIR/bin"
  cat > "$TMP_DIR/bin/cargo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "$*" >> "$TMP_DIR/calls.log"
if [[ "$1" == "nextest" && "$2" == "--version" ]]; then
  exit 0
fi
exit 0
EOF
  chmod +x "$TMP_DIR/bin/cargo"

  run env PATH="$TMP_DIR/bin:$PATH" TMP_DIR="$TMP_DIR" "$ROOT/scripts/check-tests.sh" --workspace
  [ "$status" -eq 0 ]
  run rg '^nextest run --workspace$' "$TMP_DIR/calls.log"
  [ "$status" -eq 0 ]
}

@test "check-tests falls back to cargo test when nextest is unavailable" {
  mkdir -p "$TMP_DIR/bin"
  cat > "$TMP_DIR/bin/cargo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "$*" >> "$TMP_DIR/calls.log"
if [[ "$1" == "nextest" && "$2" == "--version" ]]; then
  exit 1
fi
exit 0
EOF
  chmod +x "$TMP_DIR/bin/cargo"

  run env PATH="$TMP_DIR/bin:$PATH" TMP_DIR="$TMP_DIR" "$ROOT/scripts/check-tests.sh" --workspace
  [ "$status" -eq 0 ]
  run rg '^test --workspace$' "$TMP_DIR/calls.log"
  [ "$status" -eq 0 ]
}

