#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
  TMP_REPO="$(mktemp -d)"
  cd "$TMP_REPO"
  git init -q
  git config user.name "Test User"
  git config user.email "test@example.com"
  mkdir -p scripts docs/tasks
  cp "$ROOT/scripts/tasks.sh" scripts/tasks.sh
  chmod +x scripts/tasks.sh
  cat > docs/tasks/tasks.csv <<'EOF'
id,type,title,source,status,blocked_by,notes
TASK-A,tooling,Task A,docs/tasks/README.md,planned,,initial
EOF
  cat > docs/tasks/README.md <<'EOF'
# Tasks
- TASK-A
- TASK-NEW
EOF
}

teardown() {
  rm -rf "$TMP_REPO"
}

@test "upsert updates existing task fields" {
  run scripts/tasks.sh --upsert TASK-A --status done --notes "finished"
  [ "$status" -eq 0 ]
  run rg '^TASK-A,tooling,Task A,docs/tasks/README.md,done,,finished$' docs/tasks/tasks.csv
  [ "$status" -eq 0 ]
}

@test "upsert inserts new task when required fields provided" {
  run scripts/tasks.sh --upsert TASK-NEW --type docs --title "Task New" --source docs/tasks/README.md --status planned --notes "queued"
  [ "$status" -eq 0 ]
  run rg '^TASK-NEW,docs,Task New,docs/tasks/README.md,planned,,queued$' docs/tasks/tasks.csv
  [ "$status" -eq 0 ]
}

@test "upsert rejects invalid status" {
  run scripts/tasks.sh --upsert TASK-A --status nope
  [ "$status" -ne 0 ]
}

@test "upsert requires fields for new task insertion" {
  run scripts/tasks.sh --upsert TASK-MISSING --status done
  [ "$status" -ne 0 ]
  [[ "$output" == *"--type is required"* ]]
}
