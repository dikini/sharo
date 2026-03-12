#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

explicit_range=""
remote_name=""
remote_url=""
push_spec_file=""
declare -a diff_ranges=()
declare -a commit_ranges=()

usage() {
  cat <<'USAGE'
Usage:
  scripts/check-prepush-policy.sh [--range <git-range>] [--remote-name <name>] [--remote-url <url>] [--push-spec-file <path>]
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --range)
      shift
      if [[ $# -eq 0 ]]; then
        echo "pre-push-policy: --range requires a value" >&2
        exit 2
      fi
      explicit_range="$1"
      shift
      ;;
    --remote-name)
      shift
      if [[ $# -eq 0 ]]; then
        echo "pre-push-policy: --remote-name requires a value" >&2
        exit 2
      fi
      remote_name="$1"
      shift
      ;;
    --remote-url)
      shift
      if [[ $# -eq 0 ]]; then
        echo "pre-push-policy: --remote-url requires a value" >&2
        exit 2
      fi
      remote_url="$1"
      shift
      ;;
    --push-spec-file)
      shift
      if [[ $# -eq 0 ]]; then
        echo "pre-push-policy: --push-spec-file requires a value" >&2
        exit 2
      fi
      push_spec_file="$1"
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "pre-push-policy: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

resolve_ranges() {
  if [[ -n "$explicit_range" ]]; then
    diff_ranges=("$explicit_range")
    commit_ranges=("$explicit_range")
    return 0
  fi

  if [[ -n "$push_spec_file" && -f "$push_spec_file" ]]; then
    while IFS=' ' read -r _local_ref local_sha _remote_ref remote_sha; do
      [[ -n "${local_sha:-}" ]] || continue
      if [[ "$local_sha" == "0000000000000000000000000000000000000000" ]]; then
        continue
      fi
      if [[ "$remote_sha" == "0000000000000000000000000000000000000000" ]]; then
        diff_ranges+=("${local_sha}^!")
        commit_ranges+=("${local_sha}^!")
      else
        diff_ranges+=("${remote_sha}...${local_sha}")
        commit_ranges+=("${remote_sha}..${local_sha}")
      fi
    done <"$push_spec_file"
    if [[ "${#diff_ranges[@]}" -gt 0 ]]; then
      return 0
    fi
  fi

  local upstream_ref
  upstream_ref="$(git rev-parse --abbrev-ref --symbolic-full-name '@{upstream}' 2>/dev/null || true)"
  if [[ -n "$upstream_ref" ]]; then
    diff_ranges=("${upstream_ref}...HEAD")
    commit_ranges=("${upstream_ref}..HEAD")
    return 0
  fi

  if git rev-parse --verify --quiet origin/main >/dev/null 2>&1; then
    diff_ranges=("origin/main...HEAD")
    commit_ranges=("origin/main..HEAD")
    return 0
  fi

  echo "pre-push-policy: could not resolve push range (missing upstream and origin/main)" >&2
  exit 1
}

print_selected_ranges() {
  if [[ "${#diff_ranges[@]}" -eq 1 ]]; then
    echo "pre-push-policy: using range ${diff_ranges[0]}"
    return
  fi
  echo "pre-push-policy: using ${#diff_ranges[@]} push ranges"
  for range in "${diff_ranges[@]}"; do
    echo "pre-push-policy: range $range"
  done
}

collect_changed_files_in_ranges() {
  local pathspec=("$@")
  local range
  for range in "${diff_ranges[@]}"; do
    git diff --name-only "$range" -- "${pathspec[@]}"
  done | sed '/^$/d' | sort -u
}

run_docs_lint_in_ranges() {
  mapfile -t docs_files < <(
    collect_changed_files_in_ranges README.md docs AGENTS.md |
      sed '/^$/d' |
      rg '(^README\.md$|^AGENTS\.md$|\.md$)' |
      sort -u
  )

  if [[ "${#docs_files[@]}" -eq 0 ]]; then
    echo "doc-lint: no docs changed in range"
    return 0
  fi

  for file_path in "${docs_files[@]}"; do
    [[ -f "$file_path" ]] || continue
    scripts/doc-lint.sh --path "$file_path" --strict-new
  done
}

run_doc_terms_in_ranges() {
  mapfile -t docs_files < <(
    collect_changed_files_in_ranges README.md docs AGENTS.md |
      sed '/^$/d' |
      rg '(^README\.md$|^AGENTS\.md$|\.md$)' |
      sort -u
  )

  if [[ "${#docs_files[@]}" -eq 0 ]]; then
    echo "doc-terms: no docs changed in range"
    return 0
  fi

  for file_path in "${docs_files[@]}"; do
    [[ -f "$file_path" ]] || continue
    scripts/check-doc-terms.sh --path "$file_path"
  done
}

run_conventional_commits_in_ranges() {
  local commits
  commits="$(
    for range in "${commit_ranges[@]}"; do
      git rev-list --no-merges "$range"
    done | awk '!seen[$0]++'
  )"

  if [[ -z "$commits" ]]; then
    echo "pre-push-policy: no non-merge commits in range"
    return 0
  fi

  while IFS= read -r sha; do
    local tmp
    tmp="$(mktemp)"
    git log -1 --pretty=%B "$sha" | head -n1 >"$tmp"
    echo "pre-push-policy: checking commit $sha: $(cat "$tmp")"
    scripts/check-conventional-commit.sh "$tmp"
    rm -f "$tmp"
  done <<<"$commits"
}

enforce_changelog_in_ranges() {
  if collect_changed_files_in_ranges . | grep -qx 'CHANGELOG.md'; then
    echo "pre-push-policy: CHANGELOG.md updated in range"
    return 0
  fi
  echo "pre-push-policy: CHANGELOG.md must be updated in push range" >&2
  exit 1
}

resolve_ranges
print_selected_ranges
if [[ -n "$remote_name" ]]; then
  echo "pre-push-policy: remote $remote_name ${remote_url:-}"
fi

scripts/check-fast-feedback.sh --all --no-marker
scripts/check-shell-quality.sh --all
scripts/check-workflows.sh
for range in "${diff_ranges[@]}"; do
  scripts/check-doc-portability.sh --range "$range"
done

if collect_changed_files_in_ranges . | rg -n '(^Cargo\.lock$|(^|/)Cargo\.toml$)' >/dev/null 2>&1; then
  scripts/check-dependencies-security.sh
else
  echo "pre-push-policy: skipping dependency-security (no Cargo inputs changed in range)"
fi

run_docs_lint_in_ranges
run_doc_terms_in_ranges
for range in "${diff_ranges[@]}"; do
  scripts/check-sync-manifest.sh --range "$range"
  scripts/check-tasks-sync.sh --range "$range"
  scripts/check-flaky-regressions.sh --range "$range"
done
run_conventional_commits_in_ranges
enforce_changelog_in_ranges

echo "pre-push-policy: OK"
