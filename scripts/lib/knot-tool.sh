#!/usr/bin/env bash

set -euo pipefail

knot_tool_fail() {
  echo "knot-tool: $1" >&2
  return 1
}

knot_tool_require_cmds() {
  command -v knot >/dev/null 2>&1 || knot_tool_fail "missing required command: knot"
  command -v jq >/dev/null 2>&1 || knot_tool_fail "missing required command: jq"
}

knot_tool_require_read_only_command() {
  local command_name="$1"
  case "$command_name" in
    list_notes|list_tags|list_directory|search_notes|graph_neighbors|get_note)
      return 0
      ;;
    *)
      knot_tool_fail "command is not allowed in read-only mode: $command_name"
      ;;
  esac
}

knot_tool_command_help() {
  local command_name="$1"
  knot tool "$command_name" --help
}

knot_tool_extract_schema_json() {
  local help_text="$1"
  local schema
  schema="$(printf '%s\n' "$help_text" | awk '
    /^Input schema:/ {capture=1; next}
    /^Required fields:/ {capture=0}
    capture {print}
  ')"

  if [[ -z "${schema//[[:space:]]/}" ]]; then
    knot_tool_fail "unable to parse input schema from command help"
  fi

  printf '%s\n' "$schema"
}

knot_tool_validate_payload_against_schema() {
  local payload_json="$1"
  local schema_json="$2"

  echo "$payload_json" | jq -e 'type == "object"' >/dev/null || knot_tool_fail "payload must be a json object"

  while IFS= read -r required_field; do
    [[ -z "$required_field" ]] && continue
    echo "$payload_json" | jq -e --arg key "$required_field" 'has($key) and .[$key] != null' >/dev/null || knot_tool_fail "payload missing required field: $required_field"
  done < <(echo "$schema_json" | jq -r '.required[]?')

  while IFS=$'\t' read -r key type minimum maximum; do
    [[ -z "$key" ]] && continue

    if ! echo "$payload_json" | jq -e --arg key "$key" 'has($key)' >/dev/null; then
      continue
    fi

    case "$type" in
      string)
        echo "$payload_json" | jq -e --arg key "$key" '.[$key] | type == "string"' >/dev/null || knot_tool_fail "field '$key' must be a string"
        ;;
      integer)
        echo "$payload_json" | jq -e --arg key "$key" '.[$key] | type == "number" and (floor == .)' >/dev/null || knot_tool_fail "field '$key' must be an integer"
        if [[ "$minimum" != "null" ]]; then
          echo "$payload_json" | jq -e --arg key "$key" --argjson minimum "$minimum" '.[$key] >= $minimum' >/dev/null || knot_tool_fail "field '$key' must be >= $minimum"
        fi
        if [[ "$maximum" != "null" ]]; then
          echo "$payload_json" | jq -e --arg key "$key" --argjson maximum "$maximum" '.[$key] <= $maximum' >/dev/null || knot_tool_fail "field '$key' must be <= $maximum"
        fi
        ;;
      *)
        knot_tool_fail "unsupported schema type '$type' for field '$key'"
        ;;
    esac
  done < <(echo "$schema_json" | jq -r '.properties | to_entries[]? | [.key, (.value.type // ""), (.value.minimum // "null"), (.value.maximum // "null")] | @tsv')
}

knot_tool_call_json() {
  local command_name="$1"
  local payload_json="$2"

  knot_tool_require_cmds
  knot_tool_require_read_only_command "$command_name"

  local help_text schema_json
  help_text="$(knot_tool_command_help "$command_name")"
  schema_json="$(knot_tool_extract_schema_json "$help_text")"
  knot_tool_validate_payload_against_schema "$payload_json" "$schema_json"

  knot tool "$command_name" --json "$payload_json"
}

knot_tool_extract_note_content() {
  local response_json="$1"
  echo "$response_json" | jq -er '
    .content
    // .markdown
    // .note.content
    // .note.markdown
    // .data.content
    // .data.markdown
  '
}
