#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
  TMP_ROOT="$(mktemp -d)"
  REMOTE_REPO="$TMP_ROOT/remote.git"
  TEST_REPO="$TMP_ROOT/repo"

  git init --bare -q "$REMOTE_REPO"
  git init -q "$TEST_REPO"
  cd "$TEST_REPO"
  git config user.name "Test User"
  git config user.email "test@example.com"

  cat > README.md <<'EOF'
# temp
EOF
  cat > CHANGELOG.md <<'EOF'
# Changelog
EOF
  cat > Cargo.toml <<'EOF'
[workspace]
members = []
EOF

  git add README.md CHANGELOG.md Cargo.toml
  git commit -q -m "feat: initial"
  git branch -M main
  git remote add origin "$REMOTE_REPO"
  git push -q -u origin main

  mkdir -p bin scripts .githooks docs
  cat > bin/cargo <<'EOF'
#!/usr/bin/env bash
echo "cargo $*" >> "$PWD/.cargo-calls"
exit 0
EOF
  chmod +x bin/cargo
  export PATH="$PWD/bin:$PATH"

  cp "$ROOT/scripts/check-prepush-policy.sh" scripts/check-prepush-policy.sh
  cp "$ROOT/scripts/check-doc-portability.sh" scripts/check-doc-portability.sh
  cp "$ROOT/scripts/check-flaky-regressions.sh" scripts/check-flaky-regressions.sh
  cp "$ROOT/.githooks/pre-push" .githooks/pre-push
  chmod +x scripts/check-prepush-policy.sh scripts/check-doc-portability.sh \
    scripts/check-flaky-regressions.sh .githooks/pre-push

  mkdir -p scripts
  for script in \
    check-fast-feedback.sh \
    check-shell-quality.sh \
    check-workflows.sh \
    check-dependencies-security.sh \
    check-sync-manifest.sh \
    check-tasks-sync.sh \
    check-conventional-commit.sh \
    doc-lint.sh \
    check-doc-terms.sh; do
    cat > "scripts/$script" <<'EOF'
#!/usr/bin/env bash
echo "$(basename "$0"): $*" >> .invocations
exit 0
EOF
    chmod +x "scripts/$script"
  done
}

teardown() {
  rm -rf "$TMP_ROOT"
}

@test "pre-push hook delegates to pre-push policy script" {
  cat > scripts/check-prepush-policy.sh <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$*" > .hook-call
cat "${6#--push-spec-file }" >/dev/null 2>&1 || true
exit 0
EOF
  chmod +x scripts/check-prepush-policy.sh

  run bash -c "printf 'refs/heads/main %s refs/heads/main %s\n' \"\$(git rev-parse HEAD)\" \"\$(git rev-parse HEAD^)\" | .githooks/pre-push origin '$REMOTE_REPO'" 
  [ "$status" -eq 0 ]
  [[ "$(cat .hook-call)" == *"--remote-name origin --remote-url $REMOTE_REPO"* ]]
  [[ "$(cat .hook-call)" == *"--push-spec-file"* ]]
}

@test "pre-push policy uses upstream range when tracking branch exists" {
  git switch -q -c feature --track origin/main
  echo "update" >> CHANGELOG.md
  git add CHANGELOG.md
  git commit -q -m "docs: update changelog"

  run scripts/check-prepush-policy.sh
  [ "$status" -eq 0 ]
  [[ "$output" == *"pre-push-policy: using range origin/main...HEAD"* ]]
}

@test "pre-push policy falls back to origin main when no upstream exists" {
  git switch -q -c local-only
  echo "update" >> CHANGELOG.md
  git add CHANGELOG.md
  git commit -q -m "docs: local changelog"

  run scripts/check-prepush-policy.sh
  [ "$status" -eq 0 ]
  [[ "$output" == *"pre-push-policy: using range origin/main...HEAD"* ]]
}

@test "pre-push policy uses pushed refs when hook input is available" {
  git switch -q -c release
  echo "release" >> CHANGELOG.md
  git add CHANGELOG.md
  git commit -q -m "docs: release note"
  local_sha="$(git rev-parse HEAD)"

  cat > push-specs.txt <<EOF
refs/heads/release $local_sha refs/heads/release 0000000000000000000000000000000000000000
EOF

  run scripts/check-prepush-policy.sh --push-spec-file push-specs.txt
  [ "$status" -eq 0 ]
  [[ "$output" == *"pre-push-policy: using range ${local_sha}^!"* ]]
}

@test "pre-push policy checks only pushed commits for rewritten refs" {
  cat > scripts/check-conventional-commit.sh <<'EOF'
#!/usr/bin/env bash
echo "commit-check: $(cat "$1")"
exit 0
EOF
  chmod +x scripts/check-conventional-commit.sh

  git switch -q -c rewrite
  echo "rewrite-1" >> CHANGELOG.md
  git add CHANGELOG.md
  git commit -q -m "docs: rewrite old"
  remote_sha="$(git rev-parse HEAD)"

  echo "rewrite-2" >> CHANGELOG.md
  git add CHANGELOG.md
  git commit -q -m "docs: pushed commit"
  local_sha="$(git rev-parse HEAD)"

  git reset --hard -q HEAD~1
  echo "replacement" >> CHANGELOG.md
  git add CHANGELOG.md
  git commit -q -m "docs: replacement commit"
  local_sha="$(git rev-parse HEAD)"

  cat > push-specs.txt <<EOF
refs/heads/rewrite $local_sha refs/heads/rewrite $remote_sha
EOF

  run scripts/check-prepush-policy.sh --push-spec-file push-specs.txt
  [ "$status" -eq 0 ]
  [[ "$output" == *"pre-push-policy: using range ${remote_sha}...${local_sha}"* ]]
  [[ "$output" == *"commit-check: docs: replacement commit"* ]]
  [[ "$output" != *"commit-check: docs: rewrite old"* ]]
  [[ "$output" != *"commit-check: docs: pushed commit"* ]]
}

@test "pre-push policy checks all pushed refs" {
  cat > scripts/check-conventional-commit.sh <<'EOF'
#!/usr/bin/env bash
echo "commit-check: $(cat "$1")"
exit 0
EOF
  chmod +x scripts/check-conventional-commit.sh

  git switch -q -c release-a
  echo "a" >> CHANGELOG.md
  git add CHANGELOG.md
  git commit -q -m "docs: release a"
  sha_a="$(git rev-parse HEAD)"

  git switch -q main
  git switch -q -c release-b
  echo "b" >> CHANGELOG.md
  git add CHANGELOG.md
  git commit -q -m "docs: release b"
  sha_b="$(git rev-parse HEAD)"

  cat > push-specs.txt <<EOF
refs/heads/release-a $sha_a refs/heads/release-a 0000000000000000000000000000000000000000
refs/heads/release-b $sha_b refs/heads/release-b 0000000000000000000000000000000000000000
EOF

  run scripts/check-prepush-policy.sh --push-spec-file push-specs.txt
  [ "$status" -eq 0 ]
  [[ "$output" == *"commit-check: docs: release a"* ]]
  [[ "$output" == *"commit-check: docs: release b"* ]]
}

@test "pre-push policy runs dependency checks only when cargo inputs change" {
  cat > scripts/check-dependencies-security.sh <<'EOF'
#!/usr/bin/env bash
echo "dependency-security: called"
exit 0
EOF
  chmod +x scripts/check-dependencies-security.sh

  echo "note" >> README.md
  echo "entry" >> CHANGELOG.md
  git add README.md CHANGELOG.md
  git commit -q -m "docs: note"

  run scripts/check-prepush-policy.sh --range HEAD^!
  [ "$status" -eq 0 ]
  [[ "$output" == *"pre-push-policy: skipping dependency-security"* ]]
  [[ "$output" != *"dependency-security: called"* ]]

  cat > Cargo.lock <<'EOF'
version = 3
EOF
  echo "entry 2" >> CHANGELOG.md
  git add Cargo.lock CHANGELOG.md
  git commit -q -m "build: touch cargo lock"

  run scripts/check-prepush-policy.sh --range HEAD^!
  [ "$status" -eq 0 ]
  [[ "$output" == *"dependency-security: called"* ]]
}

@test "doc portability rejects machine-local and worktree-local paths" {
  cat > docs/portable.md <<'EOF'
# Portable

- [bad](/home/dikini/Projects/sharo/docs/specs/mvp.md)
- [worktree](/tmp/sharo/.worktrees/workflow/docs/specs/mvp.md)
EOF

  run scripts/check-doc-portability.sh --path docs/portable.md
  [ "$status" -ne 0 ]
  [[ "$output" == *"machine-local path"* ]]
  [[ "$output" == *"worktree-local path"* ]]
}

@test "doc portability ignores prose examples and checks README scope" {
  cat > README.md <<'EOF'
# Readme

This prose mentions /home/example/project but not as a markdown link.
- [bad](/home/dikini/Projects/sharo/docs/specs/mvp.md)
EOF

  run scripts/check-doc-portability.sh --path README.md
  [ "$status" -ne 0 ]
  [[ "$output" == *"machine-local path"* ]]
  [[ "$output" != *"This prose mentions"* ]]
}

@test "pre-push policy includes README in range-based docs checks" {
  echo "[bad phrase fix](docs/specs/mvp.md)" >> README.md
  echo "entry" >> CHANGELOG.md
  git add README.md CHANGELOG.md
  git commit -q -m "docs: touch readme"

  run scripts/check-prepush-policy.sh --range HEAD^!
  [ "$status" -eq 0 ]
  run rg 'doc-lint.sh: --path README.md --strict-new' .invocations
  [ "$status" -eq 0 ]
  run rg 'check-doc-terms.sh: --path README.md' .invocations
  [ "$status" -eq 0 ]
}

@test "flaky regressions skip unrelated changes and run for daemon paths" {
  mkdir -p docs
  echo "note" > docs/notes.md
  git add docs/notes.md
  git commit -q -m "docs: add note"

  run scripts/check-flaky-regressions.sh --range HEAD^!
  [ "$status" -eq 0 ]
  [[ "$output" == *"flaky-regressions: no daemon-impacting files changed, skipping"* ]]
  [ ! -f .cargo-calls ]

  mkdir -p crates/sharo-daemon/src
  echo "// change" > crates/sharo-daemon/src/demo.rs
  git add crates/sharo-daemon/src/demo.rs
  git commit -q -m "test: touch daemon path"

  run scripts/check-flaky-regressions.sh --range HEAD^! --iterations 2
  [ "$status" -eq 0 ]
  [[ "$output" == *"flaky-regressions: iteration 2/2"* ]]
  run rg 'duplicate_submit_during_inflight_reasoning_does_not_double_execute_provider' .cargo-calls
  [ "$status" -eq 0 ]
  run rg 'ctrl_c_waits_for_inflight_request_completion' .cargo-calls
  [ "$status" -eq 0 ]
}
