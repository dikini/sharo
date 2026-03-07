#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

target_dir="backbone/project-template"
clean=true

usage() {
  cat <<'USAGE'
Usage:
  scripts/extract-backbone.sh [--target <path>] [--no-clean]

Options:
  --target <path>  Output directory (default: backbone/project-template)
  --no-clean       Do not remove target before extraction.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)
      shift
      [[ $# -gt 0 ]] || {
        echo "extract-backbone: --target requires a value" >&2
        exit 2
      }
      target_dir="$1"
      shift
      ;;
    --no-clean)
      clean=false
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "extract-backbone: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

abs_target="$ROOT/$target_dir"

if [[ "$clean" == true && -d "$abs_target" ]]; then
  rm -rf "$abs_target"
fi

mkdir -p "$abs_target"

copy_file() {
  local src="$1"
  local dst="$abs_target/$2"
  mkdir -p "$(dirname "$dst")"
  cp "$ROOT/$src" "$dst"
}

copy_files() {
  local src
  for src in "$@"; do
    copy_file "$src" "$src"
  done
}

copy_files \
  .githooks/commit-msg \
  scripts/bootstrap-dev.sh \
  scripts/check-changelog-staged.sh \
  scripts/check-conventional-commit.sh \
  scripts/check-dependencies-security.sh \
  scripts/check-doc-terms.sh \
  scripts/check-fast-feedback-marker.sh \
  scripts/check-merge-result.sh \
  scripts/check-rust-policy.sh \
  scripts/check-rust-tests.sh \
  scripts/check-shell-quality.sh \
  scripts/check-sync-manifest.sh \
  scripts/check-tasks-registry.sh \
  scripts/check-tasks-sync.sh \
  scripts/check-tests.sh \
  scripts/check-workflows.sh \
  scripts/doc-lint.sh \
  scripts/doc-new.sh \
  scripts/doc-start.sh \
  scripts/init-repo.sh \
  scripts/install-bats.sh \
  scripts/install-hooks.sh \
  scripts/run-shell-tests.sh \
  scripts/sync-check.sh \
  scripts/tasks.sh \
  scripts/tests/test-bootstrap-dev.bats \
  scripts/tests/test-check-dependencies-security.bats \
  scripts/tests/test-check-merge-result.bats \
  scripts/tests/test-check-rust-hygiene.bats \
  scripts/tests/test-check-shell-quality.bats \
  scripts/tests/test-check-tests.bats \
  scripts/tests/test-check-workflows.bats \
  scripts/tests/test-doc-tools.bats \
  scripts/tests/test-fast-feedback-marker.bats \
  scripts/tests/test-init-repo.bats \
  scripts/tests/test-rust-policy.bats \
  scripts/tests/test-sync-tools.bats \
  scripts/tests/test-tasks-tooling.bats \
  docs/templates/AGENTS.template.md \
  docs/templates/CHANGELOG.template.md \
  docs/templates/README.md \
  docs/templates/README.template.md \
  docs/templates/spec.template.md \
  docs/templates/plan.template.md \
  docs/templates/examples/prompt-contract-minimal.md \
  docs/sync/README.md \
  docs/sync/sync-manifest.template.json \
  docs/sync/sync-evidence.template.md \
  docs/sync/examples/valid.manifest.json \
  scripts/tests/sync/invalid.missing-sync-id.manifest.json \
  audit.toml \
  deny.toml

mkdir -p "$abs_target/.githooks" "$abs_target/.github/workflows" "$abs_target/docs/tasks"

cat >"$abs_target/.githooks/pre-commit" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

if ! scripts/check-fast-feedback-marker.sh; then
  echo "pre-commit: refreshing fast-feedback marker" >&2
  scripts/check-fast-feedback.sh
  scripts/check-fast-feedback-marker.sh
fi
scripts/check-changelog-staged.sh
scripts/check-rust-policy.sh
scripts/check-rust-tests.sh --changed
scripts/check-sync-manifest.sh --changed
scripts/doc-lint.sh --changed --strict-new
scripts/check-doc-terms.sh --changed
scripts/check-tasks-registry.sh
scripts/check-tasks-sync.sh --changed
EOF

cat >"$abs_target/scripts/check-fast-feedback.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

mode="changed"
write_marker=true
git_dir="$(git rev-parse --git-dir)"
marker_file="$git_dir/.fast-feedback.ok"

usage() {
  cat <<'USAGE'
Usage:
  scripts/check-fast-feedback.sh
  scripts/check-fast-feedback.sh --changed
  scripts/check-fast-feedback.sh --all
  scripts/check-fast-feedback.sh --no-marker
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --changed)
      mode="changed"
      shift
      ;;
    --all)
      mode="all"
      shift
      ;;
    --no-marker)
      write_marker=false
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "fast-feedback: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

content_sha() {
  mapfile -t paths < <({
    git diff --name-only
    git diff --cached --name-only
    git ls-files --others --exclude-standard
  } | sed '/^$/d' | sort -u)

  if [[ "${#paths[@]}" -eq 0 ]]; then
    printf '' | sha256sum | awk '{print $1}'
    return
  fi

  {
    for p in "${paths[@]}"; do
      if [[ -e "$p" ]]; then
        hash="$(git hash-object -- "$p" 2>/dev/null || printf '__nonregular__')"
      else
        hash="__deleted__"
      fi
      printf '%s\t%s\n' "$p" "$hash"
    done
  } | sha256sum | awk '{print $1}'
}

scripts/doc-lint.sh --changed --strict-new
scripts/check-doc-terms.sh --changed
scripts/check-workflows.sh --warn-missing
scripts/check-shell-quality.sh --changed --warn-missing
scripts/check-tasks-registry.sh
scripts/check-tasks-sync.sh --changed
scripts/check-rust-policy.sh
if [[ "$mode" == "all" ]]; then
  scripts/check-rust-tests.sh --all
  scripts/run-shell-tests.sh --all
else
  scripts/check-rust-tests.sh --changed
  scripts/run-shell-tests.sh --changed
fi
scripts/check-sync-manifest.sh --changed

if [[ "$write_marker" == true ]]; then
  {
    echo "timestamp_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "head=$(git rev-parse HEAD)"
    echo "content_sha=$(content_sha)"
  } >"$marker_file"
  echo "fast-feedback: marker updated at $marker_file"
fi

echo "fast-feedback: OK"
EOF

cat >"$abs_target/scripts/check-rust-hygiene.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

strict_mode=false
check_target="all"
baseline_ref="origin/main"
semver_manifest="${RUST_HYGIENE_SEMVER_MANIFEST:-}"

usage() {
  cat <<'USAGE'
Usage:
  scripts/check-rust-hygiene.sh --advisory|--strict [--check all|udeps|msrv|semver] [--baseline-ref <git-ref>] [--semver-manifest <path>]
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --advisory)
      strict_mode=false
      shift
      ;;
    --strict)
      strict_mode=true
      shift
      ;;
    --check)
      shift
      [[ $# -gt 0 ]] || { echo "rust-hygiene: --check requires a value" >&2; exit 2; }
      check_target="$1"
      shift
      ;;
    --baseline-ref)
      shift
      [[ $# -gt 0 ]] || { echo "rust-hygiene: --baseline-ref requires a value" >&2; exit 2; }
      baseline_ref="$1"
      shift
      ;;
    --semver-manifest)
      shift
      [[ $# -gt 0 ]] || { echo "rust-hygiene: --semver-manifest requires a value" >&2; exit 2; }
      semver_manifest="$1"
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "rust-hygiene: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

warn_or_fail() {
  local message="$1"
  if [[ "$strict_mode" == true ]]; then
    echo "rust-hygiene: $message" >&2
    exit 1
  fi
  echo "rust-hygiene: warning: $message"
}

run_with_mode() {
  local label="$1"
  shift
  echo "rust-hygiene: running $label"
  if "$@"; then
    return 0
  fi
  warn_or_fail "$label failed"
}

run_udeps() {
  if ! cargo udeps --version >/dev/null 2>&1; then
    warn_or_fail "cargo-udeps missing (cargo install --locked cargo-udeps)"
    return 0
  fi
  run_with_mode "cargo +nightly udeps" cargo +nightly udeps --workspace --all-targets
}

run_msrv() {
  if ! cargo msrv --version >/dev/null 2>&1; then
    warn_or_fail "cargo-msrv missing (cargo install --locked cargo-msrv)"
    return 0
  fi
  run_with_mode "cargo msrv verify" cargo msrv verify --workspace -- cargo check --workspace --all-targets
}

run_semver() {
  if ! cargo semver-checks --version >/dev/null 2>&1; then
    warn_or_fail "cargo-semver-checks missing (cargo install --locked cargo-semver-checks)"
    return 0
  fi
  if [[ -z "$semver_manifest" ]]; then
    warn_or_fail "semver manifest not set (use --semver-manifest <path> or RUST_HYGIENE_SEMVER_MANIFEST)"
    return 0
  fi
  if [[ ! -f "$semver_manifest" ]]; then
    warn_or_fail "semver manifest path not found: $semver_manifest"
    return 0
  fi
  if ! git rev-parse --verify "$baseline_ref" >/dev/null 2>&1; then
    warn_or_fail "baseline ref '$baseline_ref' not found for semver checks"
    return 0
  fi
  run_with_mode "cargo semver-checks check-release" cargo semver-checks check-release --manifest-path "$semver_manifest" --baseline-rev "$baseline_ref"
}

case "$check_target" in
  all)
    run_udeps
    run_msrv
    run_semver
    ;;
  udeps)
    run_udeps
    ;;
  msrv)
    run_msrv
    ;;
  semver)
    run_semver
    ;;
  *)
    echo "rust-hygiene: invalid --check value '$check_target'" >&2
    usage
    exit 2
    ;;
esac

echo "rust-hygiene: OK"
EOF

cat >"$abs_target/justfile" <<'EOF'
set shell := ["bash", "-euo", "pipefail", "-c"]

setup:
    scripts/bootstrap-dev.sh --apply

init-repo:
    scripts/init-repo.sh --apply

verify:
    scripts/check-fast-feedback.sh

fast-feedback:
    scripts/check-fast-feedback.sh

merge-gate:
    scripts/check-merge-result.sh

shell-quality:
    scripts/check-shell-quality.sh --all

workflow-lint:
    scripts/check-workflows.sh

rust-hygiene:
    scripts/check-rust-hygiene.sh --advisory --check all
EOF

cat >"$abs_target/.github/workflows/policy-checks.yml" <<'EOF'
name: policy-checks

on:
  push:
  pull_request:

jobs:
  policy:
    runs-on: ubuntu-latest

    steps:
      - name: Checkout
        uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Install just
        uses: taiki-e/install-action@just

      - name: Install actionlint
        uses: taiki-e/install-action@actionlint

      - name: Install shell quality tools
        run: |
          sudo apt-get update
          sudo apt-get install -y shellcheck shfmt

      - name: Make scripts executable
        run: chmod +x scripts/*.sh

      - name: Run canonical verification entrypoint
        run: just verify

      - name: Run workflow lint checks
        run: scripts/check-workflows.sh

      - name: Run shell quality checks
        run: scripts/check-shell-quality.sh --all

      - name: Install dependency security tools
        run: cargo install --locked cargo-deny cargo-audit

      - name: Run dependency and security checks
        run: scripts/check-dependencies-security.sh

      - name: Enforce Rust policy
        run: scripts/check-rust-policy.sh

      - name: Run Rust workspace tests
        run: scripts/check-rust-tests.sh --all

      - name: Install bats-core
        run: scripts/install-bats.sh >/dev/null

      - name: Run shell tests
        run: scripts/run-shell-tests.sh --all

      - name: Run docs lint (strict profile)
        run: scripts/doc-lint.sh --strict-new

      - name: Run docs terminology checks
        run: scripts/check-doc-terms.sh

      - name: Validate task registry
        run: scripts/check-tasks-registry.sh

      - name: Resolve commit range
        id: range
        run: |
          if [[ "${{ github.event_name }}" == "pull_request" ]]; then
            RANGE="origin/${{ github.base_ref }}...${{ github.sha }}"
          else
            BEFORE="${{ github.event.before }}"
            if [[ "$BEFORE" == "0000000000000000000000000000000000000000" ]]; then
              RANGE="${{ github.sha }}^!"
            else
              RANGE="$BEFORE...${{ github.sha }}"
            fi
          fi
          echo "range=$RANGE" >> "$GITHUB_OUTPUT"
          echo "Using range: $RANGE"

      - name: Enforce sync manifest policy in range
        run: scripts/check-sync-manifest.sh --range "${{ steps.range.outputs.range }}"

      - name: Enforce task registry sync in range
        run: scripts/check-tasks-sync.sh --range "${{ steps.range.outputs.range }}"
EOF

copy_file .github/workflows/merge-result-gate.yml .github/workflows/merge-result-gate.yml
copy_file .github/workflows/rust-hygiene.yml .github/workflows/rust-hygiene.yml

cat >"$abs_target/docs/tasks/README.md" <<'EOF'
# Task Registry

This directory provides deterministic task state listing for planning and deferred work.

## Format

- Registry file: `docs/tasks/tasks.csv`
- Header columns:
  - `id,type,title,source,status,blocked_by,notes`
- Status enum:
  - `planned`
  - `deferred`
  - `in_progress`
  - `done`
  - `cancelled`

## Commands

- List all: `scripts/tasks.sh`
- Summary: `scripts/tasks.sh --summary`
- Upsert task row: `scripts/tasks.sh --upsert <id> --status <status> [--type ... --title ... --source ... --blocked-by ... --notes ...]`
- Validate registry: `scripts/check-tasks-registry.sh`
- Validate sync gating (changed files): `scripts/check-tasks-sync.sh --changed`
- Bootstrap toolchain/deps: `scripts/bootstrap-dev.sh --apply`
- Canonical task runner entrypoint: `just verify`
EOF

cat >"$abs_target/docs/tasks/tasks.csv" <<'EOF'
id,type,title,source,status,blocked_by,notes
TASK-BOOTSTRAP-001,tooling,Bootstrap and verification baseline,docs/tasks/README.md,planned,,Initialize project workflow guardrails and verify local/CI checks
EOF

cat >"$abs_target/scripts/tests/test-justfile-targets.bats" <<'EOF'
#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
}

@test "justfile includes required workflow targets" {
  run rg '^init-repo:\s*$' "$ROOT/justfile"
  [ "$status" -eq 0 ]
  run rg '^verify:\s*$' "$ROOT/justfile"
  [ "$status" -eq 0 ]
  run rg '^fast-feedback:\s*$' "$ROOT/justfile"
  [ "$status" -eq 0 ]
  run rg '^merge-gate:\s*$' "$ROOT/justfile"
  [ "$status" -eq 0 ]
  run rg '^shell-quality:\s*$' "$ROOT/justfile"
  [ "$status" -eq 0 ]
  run rg '^workflow-lint:\s*$' "$ROOT/justfile"
  [ "$status" -eq 0 ]
  run rg '^rust-hygiene:\s*$' "$ROOT/justfile"
  [ "$status" -eq 0 ]
}

@test "policy checks workflow uses just verify entrypoint" {
  run rg 'run: just verify' "$ROOT/.github/workflows/policy-checks.yml"
  [ "$status" -eq 0 ]
}
EOF

cat >"$abs_target/scripts/tests/test-precommit-fast-feedback.bats" <<'EOF'
#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
  TMP_REPO="$(mktemp -d)"
  cd "$TMP_REPO"
  git init -q
  git config user.name "Test User"
  git config user.email "test@example.com"
  mkdir -p .githooks scripts
  cp "$ROOT/.githooks/pre-commit" .githooks/pre-commit
  chmod +x .githooks/pre-commit

  cat > scripts/check-changelog-staged.sh <<'INNER'
#!/usr/bin/env bash
exit 0
INNER
  cat > scripts/check-rust-policy.sh <<'INNER'
#!/usr/bin/env bash
exit 0
INNER
  cat > scripts/check-rust-tests.sh <<'INNER'
#!/usr/bin/env bash
exit 0
INNER
  cat > scripts/check-sync-manifest.sh <<'INNER'
#!/usr/bin/env bash
exit 0
INNER
  cat > scripts/doc-lint.sh <<'INNER'
#!/usr/bin/env bash
exit 0
INNER
  cat > scripts/check-doc-terms.sh <<'INNER'
#!/usr/bin/env bash
exit 0
INNER
  cat > scripts/check-tasks-registry.sh <<'INNER'
#!/usr/bin/env bash
exit 0
INNER
  cat > scripts/check-tasks-sync.sh <<'INNER'
#!/usr/bin/env bash
exit 0
INNER

  chmod +x scripts/check-changelog-staged.sh scripts/check-rust-policy.sh scripts/check-rust-tests.sh \
    scripts/check-sync-manifest.sh scripts/doc-lint.sh scripts/check-doc-terms.sh \
    scripts/check-tasks-registry.sh scripts/check-tasks-sync.sh
}

teardown() {
  rm -rf "$TMP_REPO"
}

@test "pre-commit auto-refreshes when marker is stale" {
  cat > scripts/check-fast-feedback-marker.sh <<'INNER'
#!/usr/bin/env bash
if [[ -f .marker_ok ]]; then
  echo "fast-feedback-marker: OK"
  exit 0
fi
echo "fast-feedback-marker: stale"
exit 1
INNER
  cat > scripts/check-fast-feedback.sh <<'INNER'
#!/usr/bin/env bash
touch .marker_ok
exit 0
INNER
  chmod +x scripts/check-fast-feedback-marker.sh scripts/check-fast-feedback.sh

  run .githooks/pre-commit
  [ "$status" -eq 0 ]
}
EOF

cat >"$abs_target/README.md" <<'EOF'
# Project Backbone Template

This directory is a standalone starter backbone extracted from a source repository.

## Goal

- Provide a reusable project structure for development and maintenance.
- Encode process knowledge in scripts, hooks, templates, and CI workflows.
- Keep setup and verification deterministic.

## Included

- `scripts/`: workflow checks, docs tooling, bootstrap/install helpers, task registry tools.
- `docs/templates/`: strict-profile templates for specs, plans, changelog, README, and AGENTS.
- `docs/tasks/`: task registry baseline (`tasks.csv`) and usage guide.
- `docs/sync/`: sync manifest templates and validators.
- `.githooks/`: `pre-commit` and `commit-msg` policy hooks.
- `.github/workflows/`: policy, merge-result, and rust-hygiene workflow baselines.
- `justfile`: canonical task-runner entrypoints.

## Bootstrap

```bash
scripts/bootstrap-dev.sh --apply
scripts/init-repo.sh --apply
just verify
```

## Notes

- This backbone intentionally removes `sharo`-specific runtime checks from fast feedback.
- Rust hygiene semver checks are genericized via:
  - `scripts/check-rust-hygiene.sh --semver-manifest <path>`
  - or `RUST_HYGIENE_SEMVER_MANIFEST=<path>`
EOF

sed -i \
  -e 's|knot://sharo/|knot://project/|g' \
  "$abs_target/docs/sync/sync-manifest.template.json" \
  "$abs_target/docs/sync/examples/valid.manifest.json" \
  "$abs_target/scripts/tests/sync/invalid.missing-sync-id.manifest.json"

sed -i \
  -e 's|Project: sharo|Project: <project-name>|g' \
  "$abs_target/docs/templates/examples/prompt-contract-minimal.md"

sed -i \
  -e 's|/home/dikini/Projects/sharo/docs/specs/vault-sync-protocol.md|docs/specs/vault-sync-protocol.md|g' \
  -e 's|/home/dikini/Projects/sharo/docs/sync/README.md|docs/sync/README.md|g' \
  "$abs_target/docs/sync/sync-evidence.template.md"

tmp_doc_lint="$(mktemp)"
awk '
  BEGIN { skip = 0 }
  /# TEMP_GUARD: stale_mvp_spec_path/ { skip = 1; next }
  skip == 1 && /^if rg -n "docs\/plan\/mvp\\.md"/ { next }
  skip == 1 && /fail "found stale path '\''docs\/plan\/mvp\.md'\''/ { next }
  skip == 1 && /^fi$/ { skip = 0; next }
  skip == 1 { next }
  { print }
' "$abs_target/scripts/doc-lint.sh" >"$tmp_doc_lint"
mv "$tmp_doc_lint" "$abs_target/scripts/doc-lint.sh"

chmod +x \
  "$abs_target/.githooks/commit-msg" \
  "$abs_target/.githooks/pre-commit" \
  "$abs_target/scripts/"*.sh

chmod +x \
  "$abs_target/scripts/tests/test-justfile-targets.bats" \
  "$abs_target/scripts/tests/test-precommit-fast-feedback.bats"

echo "extract-backbone: wrote $target_dir"
