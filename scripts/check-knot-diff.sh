#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

source scripts/lib/knot-tool.sh

mapping_path=""
format="text"

usage() {
  cat <<'USAGE'
Usage:
  scripts/check-knot-diff.sh --mapping <mapping.csv> [--format text|json]

Mapping CSV header:
  canonical_path,knot_path
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --mapping)
      shift
      [[ $# -gt 0 ]] || { echo "knot-diff: --mapping requires a value" >&2; exit 2; }
      mapping_path="$1"
      shift
      ;;
    --format)
      shift
      [[ $# -gt 0 ]] || { echo "knot-diff: --format requires a value" >&2; exit 2; }
      format="$1"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "knot-diff: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

[[ -n "$mapping_path" ]] || { echo "knot-diff: --mapping is required" >&2; usage; exit 2; }
[[ -f "$mapping_path" ]] || { echo "knot-diff: mapping file not found: $mapping_path" >&2; exit 2; }
[[ "$format" == "text" || "$format" == "json" ]] || { echo "knot-diff: unsupported format '$format'" >&2; exit 2; }

normalize_and_hash_text() {
  awk '{ sub(/[[:space:]]+$/, ""); print }' <<<"$1" | sha256sum | awk '{print $1}'
}

normalize_and_hash_file() {
  awk '{ sub(/[[:space:]]+$/, ""); print }' "$1" | sha256sum | awk '{print $1}'
}

mismatches=0
checked=0

report_lines=()
json_records=()

add_record() {
  local kind="$1"
  local canonical_path="$2"
  local knot_path="$3"
  local details="$4"

  report_lines+=("$kind canonical_path=$canonical_path knot_path=$knot_path details=$details")
  json_records+=("$(jq -cn \
    --arg kind "$kind" \
    --arg canonical_path "$canonical_path" \
    --arg knot_path "$knot_path" \
    --arg details "$details" \
    '{kind:$kind, canonical_path:$canonical_path, knot_path:$knot_path, details:$details}')")
}

line_no=0
while IFS=',' read -r canonical_path knot_path extra; do
  line_no=$((line_no + 1))
  [[ -z "$canonical_path$knot_path$extra" ]] && continue

  if [[ "$line_no" -eq 1 ]]; then
    if [[ "$canonical_path" != "canonical_path" || "$knot_path" != "knot_path" ]]; then
      echo "knot-diff: invalid header in $mapping_path" >&2
      exit 2
    fi
    continue
  fi

  if [[ -n "${extra:-}" ]]; then
    add_record "invalid_row" "$canonical_path" "$knot_path" "unexpected third column"
    mismatches=$((mismatches + 1))
    continue
  fi

  if [[ -z "$canonical_path" || -z "$knot_path" ]]; then
    add_record "invalid_row" "$canonical_path" "$knot_path" "missing required column"
    mismatches=$((mismatches + 1))
    continue
  fi

  checked=$((checked + 1))

  if [[ ! -f "$canonical_path" ]]; then
    add_record "missing_in_repo" "$canonical_path" "$knot_path" "canonical file not found"
    mismatches=$((mismatches + 1))
    continue
  fi

  payload="$(jq -cn --arg path "$knot_path" '{path:$path}')"
  set +e
  note_response="$(knot_tool_call_json get_note "$payload" 2>&1)"
  knot_code=$?
  set -e
  if [[ "$knot_code" -ne 0 ]]; then
    add_record "missing_in_vault" "$canonical_path" "$knot_path" "$note_response"
    mismatches=$((mismatches + 1))
    continue
  fi

  set +e
  knot_content="$(knot_tool_extract_note_content "$note_response" 2>/dev/null)"
  content_code=$?
  set -e
  if [[ "$content_code" -ne 0 ]]; then
    add_record "invalid_note_payload" "$canonical_path" "$knot_path" "content field is missing"
    mismatches=$((mismatches + 1))
    continue
  fi

  repo_hash="$(normalize_and_hash_file "$canonical_path")"
  knot_hash="$(normalize_and_hash_text "$knot_content")"

  if [[ "$repo_hash" != "$knot_hash" ]]; then
    add_record "hash_mismatch" "$canonical_path" "$knot_path" "repo_hash=$repo_hash knot_hash=$knot_hash"
    mismatches=$((mismatches + 1))
  fi
done < "$mapping_path"

if [[ "$format" == "json" ]]; then
  jq -cn \
    --argjson checked "$checked" \
    --argjson mismatches "$mismatches" \
    --argjson records "[$(IFS=,; echo "${json_records[*]-}")]" \
    '{checked:$checked, mismatches:$mismatches, records:$records}'
else
  echo "knot-diff: checked=$checked mismatches=$mismatches"
  if [[ "${#report_lines[@]}" -gt 0 ]]; then
    printf '%s\n' "${report_lines[@]}"
  fi
fi

if [[ "$mismatches" -gt 0 ]]; then
  echo "knot-diff: FAILED" >&2
  exit 1
fi

echo "knot-diff: OK"
