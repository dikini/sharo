#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

mode=""
force=false
project_name="$(basename "$ROOT")"

usage() {
  cat <<'USAGE'
Usage:
  scripts/init-repo.sh --check
  scripts/init-repo.sh --apply [--force] [--project <name>]

Options:
  --check           Validate whether top-level starter files exist.
  --apply           Create starter files from templates when missing.
  --force           Overwrite existing starter files (only with --apply).
  --project <name>  Project name used for README template replacement.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --check)
      mode="check"
      shift
      ;;
    --apply)
      mode="apply"
      shift
      ;;
    --force)
      force=true
      shift
      ;;
    --project)
      shift
      [[ $# -gt 0 ]] || {
        echo "init-repo: --project requires a value" >&2
        exit 2
      }
      project_name="$1"
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "init-repo: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

if [[ -z "$mode" ]]; then
  echo "init-repo: explicit mode required (--check or --apply)" >&2
  usage
  exit 2
fi

if [[ "$mode" != "apply" && "$force" == true ]]; then
  echo "init-repo: --force is only valid with --apply" >&2
  exit 2
fi

readme_template="docs/templates/README.template.md"
agents_template="docs/templates/AGENTS.template.md"
readme_target="README.md"
agents_target="AGENTS.md"

[[ -f "$readme_template" ]] || {
  echo "init-repo: missing template $readme_template" >&2
  exit 1
}
[[ -f "$agents_template" ]] || {
  echo "init-repo: missing template $agents_template" >&2
  exit 1
}

write_readme() {
  sed \
    -e "s|<project-name>|$project_name|g" \
    -e "s|<workspace>|$project_name|g" \
    "$readme_template" >"$readme_target"
}

write_agents() {
  cp "$agents_template" "$agents_target"
}

check_file() {
  local path="$1"
  if [[ -f "$path" ]]; then
    echo "init-repo: present $path"
    return 0
  fi
  echo "init-repo: missing $path" >&2
  return 1
}

if [[ "$mode" == "check" ]]; then
  failures=0
  check_file "$readme_target" || failures=$((failures + 1))
  check_file "$agents_target" || failures=$((failures + 1))
  if [[ "$failures" -gt 0 ]]; then
    echo "init-repo: FAILED (missing starter files)" >&2
    exit 1
  fi
  echo "init-repo: OK"
  exit 0
fi

if [[ -f "$readme_target" && "$force" != true ]]; then
  echo "init-repo: skip existing $readme_target (use --force to overwrite)"
else
  write_readme
  echo "init-repo: wrote $readme_target"
fi

if [[ -f "$agents_target" && "$force" != true ]]; then
  echo "init-repo: skip existing $agents_target (use --force to overwrite)"
else
  write_agents
  echo "init-repo: wrote $agents_target"
fi

echo "init-repo: OK"
