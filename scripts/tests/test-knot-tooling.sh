#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

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

run_expect_ok() {
  local label="$1"
  shift
  if ! "$@"; then
    echo "knot-tooling-tests: expected success: $label" >&2
    return 1
  fi
}

run_expect_fail() {
  local label="$1"
  shift
  set +e
  "$@"
  code=$?
  set -e
  if [[ "$code" -eq 0 ]]; then
    echo "knot-tooling-tests: expected failure: $label" >&2
    return 1
  fi
}

export PATH="$tmp/bin:$PATH"
export FAKE_VAULT_ROOT="$tmp/vault"

# Diff checker fixtures
cat > "$tmp/vault/vault/note-a.md" <<'EOF_NOTE_A'
# Note A
Shared content.
EOF_NOTE_A

cat > "$tmp/repo/docs/research/note-a.md" <<'EOF_REPO_A'
# Note A
Shared content.
EOF_REPO_A

cat > "$tmp/repo/mapping.csv" <<EOF_MAP
canonical_path,knot_path
$tmp/repo/docs/research/note-a.md,vault/note-a.md
EOF_MAP

run_expect_ok "diff checker clean" scripts/check-knot-diff.sh --mapping "$tmp/repo/mapping.csv"

cat > "$tmp/repo/docs/research/note-a.md" <<'EOF_REPO_MISMATCH'
# Note A
Changed content.
EOF_REPO_MISMATCH
run_expect_fail "diff checker mismatch" scripts/check-knot-diff.sh --mapping "$tmp/repo/mapping.csv"

cat > "$tmp/repo/missing-vault.csv" <<EOF_MISSING
canonical_path,knot_path
$tmp/repo/docs/research/note-a.md,vault/does-not-exist.md
EOF_MISSING
run_expect_fail "diff checker missing vault note" scripts/check-knot-diff.sh --mapping "$tmp/repo/missing-vault.csv"

# Research lint fixtures
cat > "$tmp/repo/docs/research/agent-research.md" <<EOF_RESEARCH
# Agent Research

Addenda:
- $tmp/repo/docs/addenda/memory.md

Markers:
- [addendum:memory]
EOF_RESEARCH

cat > "$tmp/repo/docs/addenda/memory.md" <<'EOF_ADDENDUM'
# Memory Addendum
EOF_ADDENDUM

cat > "$tmp/repo/research-registry.csv" <<EOF_REG
note_path,required_markers,required_refs
$tmp/repo/docs/research/agent-research.md,[addendum:memory],$tmp/repo/docs/addenda/memory.md
EOF_REG

run_expect_ok "research lint clean" scripts/check-research-references.sh --registry "$tmp/repo/research-registry.csv"

cat > "$tmp/repo/docs/research/agent-research.md" <<EOF_RESEARCH_BAD
# Agent Research

Addenda:
- $tmp/repo/docs/addenda/memory.md
EOF_RESEARCH_BAD
run_expect_fail "research lint missing marker" scripts/check-research-references.sh --registry "$tmp/repo/research-registry.csv"

rm -f "$tmp/repo/docs/addenda/memory.md"
run_expect_fail "research lint missing referenced file" scripts/check-research-references.sh --registry "$tmp/repo/research-registry.csv"

echo "knot-tooling-tests: OK"
