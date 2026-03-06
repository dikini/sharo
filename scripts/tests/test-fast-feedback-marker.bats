#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
  TMP_REPO="$(mktemp -d)"
  cd "$TMP_REPO"
  git init -q
  git config user.name "Test User"
  git config user.email "test@example.com"
  echo "v1" > tracked.txt
  git add tracked.txt
  git commit -qm "init"
}

teardown() {
  rm -rf "$TMP_REPO"
}

content_sha() {
  {
    git diff --name-only
    git diff --cached --name-only
    git ls-files --others --exclude-standard
  } | sed '/^$/d' | sort -u | while IFS= read -r p; do
    [[ -z "$p" ]] && continue
    if [[ -e "$p" ]]; then
      hash="$(git hash-object -- "$p" 2>/dev/null || printf '__nonregular__')"
    else
      hash="__deleted__"
    fi
    printf '%s\t%s\n' "$p" "$hash"
  done | sha256sum | awk '{print $1}'
}

write_marker() {
  {
    echo "timestamp_utc=2026-03-05T00:00:00Z"
    echo "head=$(git rev-parse HEAD)"
    echo "content_sha=$(content_sha)"
  } > .git/.fast-feedback.ok
}

@test "content fingerprint ignores stage-only state changes" {
  echo "v2" > tracked.txt
  write_marker

  run "$ROOT/scripts/check-fast-feedback-marker.sh"
  [ "$status" -eq 0 ]

  git add tracked.txt
  run "$ROOT/scripts/check-fast-feedback-marker.sh"
  [ "$status" -eq 0 ]
}

@test "content fingerprint detects real content changes" {
  write_marker
  echo "v2" > tracked.txt

  run "$ROOT/scripts/check-fast-feedback-marker.sh"
  [ "$status" -ne 0 ]
  [[ "$output" == *"content changed since marker"* ]]
}

@test "content fingerprint handles deleted changed files" {
  echo "v2" > tracked.txt
  write_marker
  rm tracked.txt

  run "$ROOT/scripts/check-fast-feedback-marker.sh"
  [ "$status" -ne 0 ]
  [[ "$output" == *"content changed since marker"* ]]
}
