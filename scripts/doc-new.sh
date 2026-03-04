#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

usage() {
  cat <<'EOF'
Usage:
  scripts/doc-new.sh spec <slug>
  scripts/doc-new.sh plan <slug>
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

case "$kind" in
  spec)
    [[ $# -eq 1 ]] || { usage; exit 2; }
    slug="$1"
    target="docs/specs/${slug}.md"
    [[ -e "$target" ]] && { echo "doc-new: exists: $target" >&2; exit 1; }
    cp "docs/templates/spec.template.md" "$target"
    sed -i \
      -e "s|<Spec Title>|$(title_case "$slug")|g" \
      -e "s|<YYYY-MM-DD>|$(date +%F)|g" \
      "$target"
    echo "$target"
    ;;
  plan)
    [[ $# -eq 1 ]] || { usage; exit 2; }
    slug="$1"
    target="docs/plans/$(date +%F)-${slug}-plan.md"
    [[ -e "$target" ]] && { echo "doc-new: exists: $target" >&2; exit 1; }
    cp "docs/templates/plan.template.md" "$target"
    sed -i "s|<Feature>|$(title_case "$slug")|g" "$target"
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
