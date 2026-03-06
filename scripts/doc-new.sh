#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

usage() {
  cat <<'EOF'
Usage:
  scripts/doc-new.sh spec <slug> [--strict-filled]
  scripts/doc-new.sh plan <slug> [--strict-filled]
  scripts/doc-new.sh changelog
EOF
}

title_case() {
  echo "$1" | tr '_-' ' ' | awk '{
    for (i = 1; i <= NF; i++) {
      $i = toupper(substr($i,1,1)) substr($i,2)
    }
    print
  }'
}

if [[ $# -lt 1 ]]; then
  usage
  exit 2
fi

kind="$1"
shift

strict_filled=false

parse_slug_and_flags() {
  local got_slug=false
  slug=""
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --strict-filled)
        strict_filled=true
        shift
        ;;
      -*)
        echo "doc-new: unknown option '$1'" >&2
        usage
        exit 2
        ;;
      *)
        if [[ "$got_slug" == true ]]; then
          echo "doc-new: unexpected argument '$1'" >&2
          usage
          exit 2
        fi
        slug="$1"
        got_slug=true
        shift
        ;;
    esac
  done

  if [[ "$got_slug" != true ]]; then
    usage
    exit 2
  fi
}

apply_strict_filled_spec() {
  local file="$1"
  sed -i \
    -e 's/^### Task N: <Task Name>$/### Task 1: Define Initial Contract/' \
    -e 's/^- <required state or dependency>$/- Define concrete prerequisite state./' \
    -e 's/^- <must remain true during and after task>$/- Preserve explicit runtime and data invariants./' \
    -e 's/^- <observable completion condition>$/- Provide observable completion evidence./' \
    -e 's/^- <test id or test name>$/- tbd_test_id/' \
    "$file"
}

apply_strict_filled_plan() {
  local file="$1"
  sed -i \
    -e 's/^### Task N: <Task Name>$/### Task 1: Define Initial Work Slice/' \
    -e 's/^- Create:$/- Create: <new-path>/' \
    -e 's/^- Modify:$/- Modify: <existing-path>/' \
    -e 's/^- Test:$/- Test: <test-path>/' \
    -e 's/^- <required state or dependency>$/- Define concrete prerequisite state./' \
    -e 's/^- <must remain true during and after task>$/- Preserve explicit runtime and data invariants./' \
    -e 's/^- <observable completion condition>$/- Provide observable completion evidence./' \
    -e 's/^- <test id or test name>$/- tbd_test_id/' \
    -e 's|^Command: `<exact command>`$|Command: `echo "replace with red-phase command"`|' \
    -e 's|^Expected: failing tests for this task only$|Expected: failing tests for this task only (replace placeholder).|' \
    -e 's|^1. <minimal change 1>$|1. Replace placeholders with concrete scope.|' \
    -e 's|^2. <minimal change 2>$|2. Add implementation details and verification commands.|' \
    -e 's|^Expected: all task tests pass$|Expected: all task tests pass (replace placeholder).|' \
    "$file"
}

case "$kind" in
  spec)
    parse_slug_and_flags "$@"
    target="docs/specs/${slug}.md"
    [[ -e "$target" ]] && { echo "doc-new: exists: $target" >&2; exit 1; }
    cp "docs/templates/spec.template.md" "$target"
    sed -i \
      -e "s|<Spec Title>|$(title_case "$slug")|g" \
      -e "s|<YYYY-MM-DD>|$(date +%F)|g" \
      "$target"
    if [[ "$strict_filled" == true ]]; then
      apply_strict_filled_spec "$target"
    fi
    echo "$target"
    ;;
  plan)
    parse_slug_and_flags "$@"
    target="docs/plans/$(date +%F)-${slug}-plan.md"
    [[ -e "$target" ]] && { echo "doc-new: exists: $target" >&2; exit 1; }
    cp "docs/templates/plan.template.md" "$target"
    sed -i "s|<Feature>|$(title_case "$slug")|g" "$target"
    if [[ "$strict_filled" == true ]]; then
      apply_strict_filled_plan "$target"
    fi
    echo "$target"
    ;;
  changelog)
    [[ $# -eq 0 ]] || { usage; exit 2; }
    target="CHANGELOG.md"
    [[ -e "$target" ]] && { echo "doc-new: exists: $target" >&2; exit 1; }
    cp "docs/templates/CHANGELOG.template.md" "$target"
    echo "$target"
    ;;
  *)
    usage
    exit 2
    ;;
esac
