#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
CSV="docs/tasks/tasks.csv"

usage() {
  cat <<'USAGE'
Usage:
  scripts/tasks.sh
  scripts/tasks.sh --status <planned|deferred|in_progress|done|cancelled>
  scripts/tasks.sh --summary
  scripts/tasks.sh --upsert <id> [--type <type>] [--title <title>] [--source <path>] [--status <status>] [--blocked-by <id>] [--notes <text>]
USAGE
}

status_filter=""
summary=false
upsert_id=""
upsert_type=""
upsert_title=""
upsert_source=""
upsert_status=""
upsert_blocked_by=""
upsert_notes=""

allowed_status() {
  case "$1" in
    planned | deferred | in_progress | done | cancelled) return 0 ;;
    *) return 1 ;;
  esac
}

csv_escape() {
  local s="$1"
  s="${s//$'\n'/ }"
  s="${s//$'\r'/ }"
  s="${s//,/;}"
  printf '%s' "$s"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --status)
      shift
      [[ $# -gt 0 ]] || {
        echo "tasks: --status requires a value" >&2
        exit 2
      }
      if [[ -n "$upsert_id" ]]; then
        upsert_status="$1"
      else
        status_filter="$1"
      fi
      shift
      ;;
    --summary)
      summary=true
      shift
      ;;
    --upsert)
      shift
      [[ $# -gt 0 ]] || {
        echo "tasks: --upsert requires a value" >&2
        exit 2
      }
      upsert_id="$1"
      shift
      ;;
    --type)
      shift
      [[ $# -gt 0 ]] || {
        echo "tasks: --type requires a value" >&2
        exit 2
      }
      upsert_type="$1"
      shift
      ;;
    --title)
      shift
      [[ $# -gt 0 ]] || {
        echo "tasks: --title requires a value" >&2
        exit 2
      }
      upsert_title="$1"
      shift
      ;;
    --source)
      shift
      [[ $# -gt 0 ]] || {
        echo "tasks: --source requires a value" >&2
        exit 2
      }
      upsert_source="$1"
      shift
      ;;
    --blocked-by)
      shift
      [[ $# -gt 0 ]] || {
        echo "tasks: --blocked-by requires a value" >&2
        exit 2
      }
      upsert_blocked_by="$1"
      shift
      ;;
    --notes)
      shift
      [[ $# -gt 0 ]] || {
        echo "tasks: --notes requires a value" >&2
        exit 2
      }
      upsert_notes="$1"
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "tasks: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

[[ -f "$CSV" ]] || {
  echo "tasks: registry not found: $CSV" >&2
  exit 1
}

if [[ -n "$upsert_id" ]]; then
  if [[ "$summary" == true || -n "$status_filter" ]]; then
    echo "tasks: --upsert cannot be combined with --summary/--status" >&2
    exit 2
  fi

  if [[ -n "$upsert_status" ]] && ! allowed_status "$upsert_status"; then
    echo "tasks: invalid status '$upsert_status'" >&2
    exit 2
  fi

  tmp="$(mktemp)"
  trap 'rm -f "$tmp"' EXIT

  found=0
  {
    IFS= read -r header
    echo "$header"
    while IFS=',' read -r id type title source status blocked_by notes; do
      if [[ "$id" == "$upsert_id" ]]; then
        found=1
        [[ -n "$upsert_type" ]] && type="$(csv_escape "$upsert_type")"
        [[ -n "$upsert_title" ]] && title="$(csv_escape "$upsert_title")"
        [[ -n "$upsert_source" ]] && source="$(csv_escape "$upsert_source")"
        [[ -n "$upsert_status" ]] && status="$upsert_status"
        [[ -n "$upsert_blocked_by" ]] && blocked_by="$(csv_escape "$upsert_blocked_by")"
        [[ -n "$upsert_notes" ]] && notes="$(csv_escape "$upsert_notes")"
      fi
      printf '%s,%s,%s,%s,%s,%s,%s\n' "$id" "$type" "$title" "$source" "$status" "$blocked_by" "$notes"
    done
  } <"$CSV" >"$tmp"

  if [[ "$found" -eq 0 ]]; then
    [[ -n "$upsert_type" ]] || {
      echo "tasks: --type is required for new task '$upsert_id'" >&2
      exit 2
    }
    [[ -n "$upsert_title" ]] || {
      echo "tasks: --title is required for new task '$upsert_id'" >&2
      exit 2
    }
    [[ -n "$upsert_source" ]] || {
      echo "tasks: --source is required for new task '$upsert_id'" >&2
      exit 2
    }
    [[ -n "$upsert_status" ]] || {
      echo "tasks: --status is required for new task '$upsert_id'" >&2
      exit 2
    }

    printf '%s,%s,%s,%s,%s,%s,%s\n' \
      "$upsert_id" \
      "$(csv_escape "$upsert_type")" \
      "$(csv_escape "$upsert_title")" \
      "$(csv_escape "$upsert_source")" \
      "$upsert_status" \
      "$(csv_escape "$upsert_blocked_by")" \
      "$(csv_escape "$upsert_notes")" >>"$tmp"
  fi

  mv "$tmp" "$CSV"
  trap - EXIT
  echo "tasks: upserted $upsert_id"
  exit 0
fi

if [[ "$summary" == true ]]; then
  awk -F',' 'NR>1{count[$5]++} END{for (s in count) printf "%s,%d\n", s, count[s]}' "$CSV" | sort
  exit 0
fi

if [[ -n "$status_filter" ]]; then
  awk -F',' -v s="$status_filter" 'NR==1 || $5==s' "$CSV"
else
  cat "$CSV"
fi
