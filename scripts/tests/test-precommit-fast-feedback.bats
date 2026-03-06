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

  cat > scripts/check-changelog-staged.sh <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  cat > scripts/check-rust-policy.sh <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  cat > scripts/check-rust-tests.sh <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  cat > scripts/check-sync-manifest.sh <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  cat > scripts/doc-lint.sh <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  cat > scripts/check-doc-terms.sh <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  cat > scripts/check-tasks-registry.sh <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  cat > scripts/check-tasks-sync.sh <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  chmod +x scripts/check-changelog-staged.sh scripts/check-rust-policy.sh scripts/check-rust-tests.sh \
    scripts/check-sync-manifest.sh scripts/doc-lint.sh scripts/check-doc-terms.sh \
    scripts/check-tasks-registry.sh scripts/check-tasks-sync.sh
}

teardown() {
  rm -rf "$TMP_REPO"
}

@test "pre-commit auto-refreshes when marker is stale" {
  cat > scripts/check-fast-feedback-marker.sh <<'EOF'
#!/usr/bin/env bash
if [[ -f .marker_ok ]]; then
  echo "fast-feedback-marker: OK"
  exit 0
fi
echo "fast-feedback-marker: stale"
exit 1
EOF
  cat > scripts/check-fast-feedback.sh <<'EOF'
#!/usr/bin/env bash
count_file=".refresh_count"
count=0
[[ -f "$count_file" ]] && count="$(cat "$count_file")"
count=$((count + 1))
echo "$count" > "$count_file"
touch .marker_ok
exit 0
EOF
  chmod +x scripts/check-fast-feedback-marker.sh scripts/check-fast-feedback.sh

  run .githooks/pre-commit
  [ "$status" -eq 0 ]
  [ "$(cat .refresh_count)" -eq 1 ]
}

@test "pre-commit fails when auto-refresh fails" {
  cat > scripts/check-fast-feedback-marker.sh <<'EOF'
#!/usr/bin/env bash
echo "fast-feedback-marker: stale"
exit 1
EOF
  cat > scripts/check-fast-feedback.sh <<'EOF'
#!/usr/bin/env bash
exit 1
EOF
  chmod +x scripts/check-fast-feedback-marker.sh scripts/check-fast-feedback.sh

  run .githooks/pre-commit
  [ "$status" -ne 0 ]
}

@test "pre-commit skips refresh when marker already valid" {
  cat > scripts/check-fast-feedback-marker.sh <<'EOF'
#!/usr/bin/env bash
echo "fast-feedback-marker: OK"
exit 0
EOF
  cat > scripts/check-fast-feedback.sh <<'EOF'
#!/usr/bin/env bash
echo "should-not-run" > .unexpected
exit 1
EOF
  chmod +x scripts/check-fast-feedback-marker.sh scripts/check-fast-feedback.sh

  run .githooks/pre-commit
  [ "$status" -eq 0 ]
  [ ! -f .unexpected ]
}
