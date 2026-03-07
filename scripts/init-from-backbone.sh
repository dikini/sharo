#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

source_dir="$ROOT/backbone/project-template"
dest_dir=""
project_name=""
force=false
do_commit=true
commit_message="chore: initialize project from backbone template"

usage() {
  cat <<'USAGE'
Usage:
  scripts/init-from-backbone.sh --dest <path> [options]

Options:
  --dest <path>            Destination directory for new repository (required).
  --project <name>         Project name for template token replacement (defaults to destination basename).
  --source <path>          Source backbone directory (default: backbone/project-template).
  --force                  Remove existing destination directory before initialization.
  --no-commit              Skip creating the initial git commit.
  --commit-message <msg>   Commit message for initial commit.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dest)
      shift
      [[ $# -gt 0 ]] || {
        echo "init-from-backbone: --dest requires a value" >&2
        exit 2
      }
      dest_dir="$1"
      shift
      ;;
    --project)
      shift
      [[ $# -gt 0 ]] || {
        echo "init-from-backbone: --project requires a value" >&2
        exit 2
      }
      project_name="$1"
      shift
      ;;
    --source)
      shift
      [[ $# -gt 0 ]] || {
        echo "init-from-backbone: --source requires a value" >&2
        exit 2
      }
      source_dir="$1"
      shift
      ;;
    --force)
      force=true
      shift
      ;;
    --no-commit)
      do_commit=false
      shift
      ;;
    --commit-message)
      shift
      [[ $# -gt 0 ]] || {
        echo "init-from-backbone: --commit-message requires a value" >&2
        exit 2
      }
      commit_message="$1"
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "init-from-backbone: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

[[ -n "$dest_dir" ]] || {
  echo "init-from-backbone: --dest is required" >&2
  usage
  exit 2
}

if [[ ! "$source_dir" = /* ]]; then
  source_dir="$ROOT/$source_dir"
fi

[[ -d "$source_dir" ]] || {
  echo "init-from-backbone: source directory not found: $source_dir" >&2
  exit 1
}

if [[ ! "$dest_dir" = /* ]]; then
  dest_dir="$(pwd)/$dest_dir"
fi

if [[ -z "$project_name" ]]; then
  project_name="$(basename "$dest_dir")"
fi

if [[ -e "$dest_dir" ]]; then
  if [[ "$force" != true ]]; then
    echo "init-from-backbone: destination exists: $dest_dir (use --force to replace)" >&2
    exit 1
  fi
  rm -rf "$dest_dir"
fi

mkdir -p "$dest_dir"
cp -a "$source_dir"/. "$dest_dir"/

(
  cd "$dest_dir"

  if [[ ! -d .git ]]; then
    git init -q
  fi

  scripts/init-repo.sh --apply --force --project "$project_name"

  if [[ "$do_commit" == true ]]; then
    if ! git config user.name >/dev/null 2>&1 || ! git config user.email >/dev/null 2>&1; then
      echo "init-from-backbone: git user identity not configured in destination; configure user.name and user.email or rerun with --no-commit" >&2
      exit 1
    fi
    git add .
    git commit -m "$commit_message"
  fi
)

echo "init-from-backbone: initialized $dest_dir"
