#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

registry_path=""

usage() {
  cat <<'USAGE'
Usage:
  scripts/check-research-references.sh --registry <registry.csv>

Registry CSV header:
  note_path,required_markers,required_refs

Format details:
  required_markers uses ';' as separator
  required_refs uses ';' as separator
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --registry)
      shift
      [[ $# -gt 0 ]] || { echo "research-lint: --registry requires a value" >&2; exit 2; }
      registry_path="$1"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "research-lint: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

[[ -n "$registry_path" ]] || { echo "research-lint: --registry is required" >&2; usage; exit 2; }
[[ -f "$registry_path" ]] || { echo "research-lint: registry file not found: $registry_path" >&2; exit 2; }

failures=0
checked=0

line_no=0
while IFS=',' read -r note_path required_markers required_refs extra; do
  line_no=$((line_no + 1))
  [[ -z "$note_path$required_markers$required_refs$extra" ]] && continue

  if [[ "$line_no" -eq 1 ]]; then
    if [[ "$note_path" != "note_path" || "$required_markers" != "required_markers" || "$required_refs" != "required_refs" ]]; then
      echo "research-lint: invalid header in $registry_path" >&2
      exit 2
    fi
    continue
  fi

  if [[ -n "${extra:-}" ]]; then
    echo "research-lint: invalid row line=$line_no details=unexpected fourth column" >&2
    failures=$((failures + 1))
    continue
  fi

  [[ -n "$note_path" ]] || { echo "research-lint: invalid row line=$line_no details=missing note_path" >&2; failures=$((failures + 1)); continue; }

  checked=$((checked + 1))

  if [[ ! -f "$note_path" ]]; then
    echo "research-lint: missing_note note_path=$note_path" >&2
    failures=$((failures + 1))
    continue
  fi

  IFS=';' read -ra markers <<< "$required_markers"
  for marker in "${markers[@]}"; do
    marker="${marker## }"
    marker="${marker%% }"
    [[ -z "$marker" ]] && continue

    if ! rg -n -F "$marker" "$note_path" >/dev/null 2>&1; then
      echo "research-lint: missing_marker note_path=$note_path marker=$marker" >&2
      failures=$((failures + 1))
    fi
  done

  IFS=';' read -ra refs <<< "$required_refs"
  for ref in "${refs[@]}"; do
    ref="${ref## }"
    ref="${ref%% }"
    [[ -z "$ref" ]] && continue

    if [[ ! -f "$ref" ]]; then
      echo "research-lint: missing_reference_file note_path=$note_path ref=$ref" >&2
      failures=$((failures + 1))
      continue
    fi

    if ! rg -n -F "$ref" "$note_path" >/dev/null 2>&1; then
      echo "research-lint: missing_reference_marker note_path=$note_path ref=$ref" >&2
      failures=$((failures + 1))
    fi
  done
done < "$registry_path"

if [[ "$failures" -gt 0 ]]; then
  echo "research-lint: checked=$checked failures=$failures" >&2
  echo "research-lint: FAILED" >&2
  exit 1
fi

echo "research-lint: checked=$checked failures=$failures"
echo "research-lint: OK"
