#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
  TMP_REPO="$(mktemp -d)"
  cd "$TMP_REPO"
  git init -q
  git config user.name "Test User"
  git config user.email "test@example.com"

  mkdir -p scripts docs/templates docs/specs docs/plans
  cp "$ROOT/scripts/doc-new.sh" scripts/doc-new.sh
  cp "$ROOT/scripts/doc-start.sh" scripts/doc-start.sh
  cp "$ROOT/scripts/doc-lint.sh" scripts/doc-lint.sh
  chmod +x scripts/doc-new.sh scripts/doc-start.sh scripts/doc-lint.sh

  cp "$ROOT/docs/templates/spec.template.md" docs/templates/spec.template.md
  cp "$ROOT/docs/templates/plan.template.md" docs/templates/plan.template.md
  cp "$ROOT/docs/templates/CHANGELOG.template.md" docs/templates/CHANGELOG.template.md
}

teardown() {
  rm -rf "$TMP_REPO"
}

@test "doc-new plan --strict-filled scaffolds strict sections" {
  run scripts/doc-new.sh plan sample --strict-filled
  [ "$status" -eq 0 ]
  plan_path="$output"
  [ -f "$plan_path" ]

  run rg '^### Task 1: Define Initial Work Slice$' "$plan_path"
  [ "$status" -eq 0 ]
  run rg '^Invariant:$' "$plan_path"
  [ "$status" -eq 0 ]
  run rg '^Command: `echo "replace with red-phase command"`$' "$plan_path"
  [ "$status" -eq 0 ]
  run rg '^## Instruction Priority$' "$plan_path"
  [ "$status" -eq 0 ]
  run rg '^## Execution Mode$' "$plan_path"
  [ "$status" -eq 0 ]
  run rg '^## Completion Gate$' "$plan_path"
  [ "$status" -eq 0 ]
}

@test "doc-new spec --strict-filled scaffolds strict sections" {
  run scripts/doc-new.sh spec sample --strict-filled
  [ "$status" -eq 0 ]
  spec_path="$output"
  [ -f "$spec_path" ]

  run rg '^### Task 1: Define Initial Contract$' "$spec_path"
  [ "$status" -eq 0 ]
  run rg '^Invariant:$' "$spec_path"
  [ "$status" -eq 0 ]
  run rg '^## Instruction Priority$' "$spec_path"
  [ "$status" -eq 0 ]
  run rg '^## Output Contract$' "$spec_path"
  [ "$status" -eq 0 ]
  run rg '^## Evidence / Verification Contract$' "$spec_path"
  [ "$status" -eq 0 ]
}

@test "doc-start applies strict-filled by default" {
  run scripts/doc-start.sh plan started
  [ "$status" -eq 0 ]
  run rg '^### Task 1: Define Initial Work Slice$' docs/plans/*-started-plan.md
  [ "$status" -eq 0 ]
  run rg '^## Task Update Contract$' docs/plans/*-started-plan.md
  [ "$status" -eq 0 ]
}

@test "doc-lint missing strict section provides strict-filled hint" {
  bad="docs/plans/$(date +%F)-bad-plan.md"
  cat > "$bad" <<'EOF'
# Bad Plan
Template-Profile: tdd-strict-v1
EOF

  run scripts/doc-lint.sh --path "$bad"
  [ "$status" -ne 0 ]
  [[ "$output" == *"--strict-filled"* ]]
}

@test "template README documents delimiter block guidance" {
  run rg '^## Delimiter Block Guidance$' "$ROOT/docs/templates/README.md"
  [ "$status" -eq 0 ]

  run rg 'Critical constraints must always be duplicated in plain language' "$ROOT/docs/templates/README.md"
  [ "$status" -eq 0 ]
}
