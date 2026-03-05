#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
  cd "$ROOT"
  MATRIX_MAP="docs/tasks/mvp-verification-matrix-map.csv"
}

@test "mvp matrix section exists" {
  run rg -n '^## 21\. Verification Matrix$' docs/specs/mvp.md
  [ "$status" -eq 0 ]
}

@test "mvp matrix includes expected columns" {
  run rg -n '^\| Invariant / Requirement \| Subsystem \| Scenario \| Verification Type \| Expected Evidence \|$' docs/specs/mvp.md
  [ "$status" -eq 0 ]
}

@test "mvp matrix has minimum required rows" {
  matrix_rows="$(awk '
    /^## 21\. Verification Matrix$/ { in_matrix=1; next }
    /^## / && in_matrix { in_matrix=0 }
    in_matrix && /^\|/ { print }
  ' docs/specs/mvp.md | tail -n +3 | wc -l | tr -d ' ')"

  [ "$matrix_rows" -ge 10 ]
}

@test "matrix_map_has_unique_row_keys" {
  run bash -lc "awk -F, 'NR>1 { print \$1 }' \"$MATRIX_MAP\" | sort | uniq -d"
  [ "$status" -eq 0 ]
  [ -z "$output" ]
}

@test "matrix_rows_have_test_binding" {
  run bash -lc "awk -F, 'NR>1 && (\$2==\"\" || \$3==\"\" || \$4==\"\") { print NR }' \"$MATRIX_MAP\""
  [ "$status" -eq 0 ]
  [ -z "$output" ]
}

@test "matrix_map_references_existing_tests" {
  while IFS=, read -r row_key test_id test_path binding_status notes; do
    [[ "$row_key" == "row_key" ]] && continue
    [[ -f "$test_path" ]]
    if [[ "$binding_status" == "implemented" || "$binding_status" == "partial" ]]; then
      if [[ "$test_path" == *.bats ]]; then
        run bash -lc "rg -n \"@test \\\".*$test_id.*\\\"\" \"$test_path\""
      else
        run bash -lc "rg -n \"fn $test_id\\b|$test_id\" \"$test_path\""
      fi
      [ "$status" -eq 0 ]
    fi
  done < "$MATRIX_MAP"
}
