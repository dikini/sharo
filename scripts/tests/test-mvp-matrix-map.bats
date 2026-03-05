#!/usr/bin/env bats

setup() {
  ROOT="$(git rev-parse --show-toplevel)"
  cd "$ROOT"
  MAP="docs/plans/2026-03-05-mvp-verification-matrix-map.md"
}

@test "matrix_map_has_unique_row_keys" {
  run bash -lc "rg '^\| [a-z0-9-]+ \|' \"$MAP\" | sed -E 's/^\| ([a-z0-9-]+) \| (.+) \| (.+) \|$/\\1|\\2|\\3/' | cut -d'|' -f1 | sort | uniq -d"
  [ "$status" -eq 0 ]
  [ -z "$output" ]
}

@test "matrix_rows_have_test_binding" {
  run bash -lc 'rg "^\| [a-z0-9-]+ \|" "'"$MAP"'" | sed -E "s/^\| ([a-z0-9-]+) \| (.+) \| (.+) \|$/\1|\2|\3/" | while IFS="|" read -r key binding _; do if [[ -z "$binding" ]]; then echo "missing binding for $key"; exit 1; fi; done'
  [ "$status" -eq 0 ]
}

@test "matrix_map_references_existing_tests" {
  run bash -lc 'rg "^\| [a-z0-9-]+ \|" "'"$MAP"'" | sed -E "s/^\| ([a-z0-9-]+) \| (.+) \| (.+) \|$/\1|\2|\3/" | while IFS="|" read -r _ binding _; do if [[ "$binding" == not-implemented:* ]]; then continue; fi; IFS=";" read -ra refs <<<"$binding"; for ref in "${refs[@]}"; do path="${ref%%#*}"; if [[ ! -f "$path" ]]; then echo "missing test file: $path"; exit 1; fi; done; done'
  [ "$status" -eq 0 ]
}
