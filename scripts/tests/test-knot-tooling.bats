#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
  cd "$ROOT"

  tmp="$(mktemp -d)"
  export TMP_KNOT_TEST_DIR="$tmp"
  export PATH="$tmp/bin:$PATH"
  export FAKE_VAULT_ROOT="$tmp/vault"

  mkdir -p "$tmp/bin" "$tmp/vault/vault" "$tmp/repo/docs/research" "$tmp/repo/docs/addenda"

  cat > "$tmp/bin/knot" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -lt 2 || "$1" != "tool" ]]; then
  echo "fake-knot: unsupported" >&2
  exit 2
fi

cmd="$2"
shift 2

if [[ "$cmd" == "get_note" && "${1-}" == "--help" ]]; then
  cat <<'HELP'
knot tool get_note - Get a note by path, including markdown content.

Usage:
  knot tool get_note --json '<payload>'
  knot tool get_note --stdin-json

Input schema:
{
  "type": "object",
  "properties": {
    "path": {
      "type": "string"
    }
  },
  "required": [
    "path"
  ]
}

Required fields: path
HELP
  exit 0
fi

if [[ "$cmd" == "get_note" && "${1-}" == "--json" ]]; then
  payload="${2-}"
  path="$(jq -er '.path' <<<"$payload")"
  file="$FAKE_VAULT_ROOT/$path"
  if [[ ! -f "$file" ]]; then
    echo "fake-knot: note not found: $path" >&2
    exit 3
  fi
  jq -n --arg path "$path" --rawfile content "$file" '{path:$path,content:$content}'
  exit 0
fi

echo "fake-knot: unsupported command: $cmd $*" >&2
exit 2
FAKE
  chmod +x "$tmp/bin/knot"
}

teardown() {
  rm -rf "$TMP_KNOT_TEST_DIR"
}

@test "diff checker clean match" {
  cat > "$TMP_KNOT_TEST_DIR/vault/vault/note-a.md" <<'EOF_NOTE_A'
# Note A
Shared content.
EOF_NOTE_A

  cat > "$TMP_KNOT_TEST_DIR/repo/docs/research/note-a.md" <<'EOF_REPO_A'
# Note A
Shared content.
EOF_REPO_A

  cat > "$TMP_KNOT_TEST_DIR/repo/mapping.csv" <<EOF_MAP
canonical_path,knot_path
$TMP_KNOT_TEST_DIR/repo/docs/research/note-a.md,vault/note-a.md
EOF_MAP

  run scripts/check-knot-diff.sh --mapping "$TMP_KNOT_TEST_DIR/repo/mapping.csv"
  [ "$status" -eq 0 ]
}

@test "diff checker detects content mismatch" {
  cat > "$TMP_KNOT_TEST_DIR/vault/vault/note-a.md" <<'EOF_NOTE_A'
# Note A
Shared content.
EOF_NOTE_A

  cat > "$TMP_KNOT_TEST_DIR/repo/docs/research/note-a.md" <<'EOF_REPO_A'
# Note A
Changed content.
EOF_REPO_A

  cat > "$TMP_KNOT_TEST_DIR/repo/mapping.csv" <<EOF_MAP
canonical_path,knot_path
$TMP_KNOT_TEST_DIR/repo/docs/research/note-a.md,vault/note-a.md
EOF_MAP

  run scripts/check-knot-diff.sh --mapping "$TMP_KNOT_TEST_DIR/repo/mapping.csv"
  [ "$status" -ne 0 ]
}

@test "research lint detects missing marker" {
  cat > "$TMP_KNOT_TEST_DIR/repo/docs/research/agent-research.md" <<EOF_RESEARCH
# Agent Research

Addenda:
- $TMP_KNOT_TEST_DIR/repo/docs/addenda/memory.md
EOF_RESEARCH

  cat > "$TMP_KNOT_TEST_DIR/repo/docs/addenda/memory.md" <<'EOF_ADDENDUM'
# Memory Addendum
EOF_ADDENDUM

  cat > "$TMP_KNOT_TEST_DIR/repo/research-registry.csv" <<EOF_REG
note_path,required_markers,required_refs
$TMP_KNOT_TEST_DIR/repo/docs/research/agent-research.md,[addendum:memory],$TMP_KNOT_TEST_DIR/repo/docs/addenda/memory.md
EOF_REG

  run scripts/check-research-references.sh --registry "$TMP_KNOT_TEST_DIR/repo/research-registry.csv"
  [ "$status" -ne 0 ]
}
