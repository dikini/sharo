#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
  TMP_REPO="$(mktemp -d)"
  cd "$TMP_REPO"
  git init -q
  git config user.name "Test User"
  git config user.email "test@example.com"

  mkdir -p scripts backbone/project-template/scripts backbone/project-template/docs/templates
  cp "$ROOT/scripts/init-from-backbone.sh" scripts/init-from-backbone.sh
  chmod +x scripts/init-from-backbone.sh
  cp "$ROOT/scripts/init-repo.sh" backbone/project-template/scripts/init-repo.sh
  chmod +x backbone/project-template/scripts/init-repo.sh
  cp "$ROOT/docs/templates/README.template.md" backbone/project-template/docs/templates/README.template.md
  cp "$ROOT/docs/templates/AGENTS.template.md" backbone/project-template/docs/templates/AGENTS.template.md
}

teardown() {
  rm -rf "$TMP_REPO"
}

@test "init-from-backbone requires destination" {
  run scripts/init-from-backbone.sh
  [ "$status" -eq 2 ]
  [[ "$output" == *"--dest is required"* ]]
}

@test "init-from-backbone can initialize without commit" {
  run scripts/init-from-backbone.sh --dest ./out/demo --project demo --no-commit
  [ "$status" -eq 0 ]
  [ -f out/demo/README.md ]
  [ -f out/demo/AGENTS.md ]
  [ -d out/demo/.git ]

  run rg '^# demo$' out/demo/README.md
  [ "$status" -eq 0 ]
}

@test "init-from-backbone creates initial commit by default" {
  HOME="$TMP_REPO/home"
  mkdir -p "$HOME"
  git config --global user.name "Backbone Bot"
  git config --global user.email "backbone@example.com"

  run scripts/init-from-backbone.sh --dest ./out/committed --project committed
  [ "$status" -eq 0 ]

  run git -C out/committed log -1 --pretty=%s
  [ "$status" -eq 0 ]
  [ "$output" = "chore: initialize project from backbone template" ]
}
