#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

mode="changed"
profile="smoke"

usage() {
  cat <<'USAGE'
Usage:
  scripts/check-fuzz.sh --smoke --changed
  scripts/check-fuzz.sh --smoke --all
  scripts/check-fuzz.sh --full --changed
  scripts/check-fuzz.sh --full --all

Options:
  --smoke     Run bounded smoke fuzzing (required unless --full is set).
  --full      Run deeper fuzzing profile (required unless --smoke is set).
  --changed   Fuzz only targets for fuzz-enabled crates touched in working tree.
  --all       Fuzz all discovered targets across all fuzz-enabled crates.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --smoke)
      profile="smoke"
      shift
      ;;
    --full)
      profile="full"
      shift
      ;;
    --changed)
      mode="changed"
      shift
      ;;
    --all)
      mode="all"
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "check-fuzz: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

if ! cargo fuzz --help >/dev/null 2>&1; then
  echo "check-fuzz: cargo-fuzz is required; install with: cargo install --locked cargo-fuzz" >&2
  exit 1
fi

collect_changed_files() {
  {
    git diff --name-only
    git diff --cached --name-only
    git ls-files --others --exclude-standard
  } | sed '/^$/d' | sort -u
}

discover_fuzz_crates() {
  find crates -mindepth 2 -maxdepth 2 -type f -path '*/fuzz/Cargo.toml' |
    sed 's#/fuzz/Cargo.toml$##' |
    sort -u
}

discover_fuzz_targets() {
  local crate_dir="$1"
  find "$crate_dir/fuzz/fuzz_targets" -maxdepth 1 -type f -name '*.rs' |
    sed -e 's#.*/##' -e 's#\.rs$##' |
    sort -u
}

should_run_crate_in_changed_mode() {
  local crate_dir="$1"
  local changed_file
  while IFS= read -r changed_file; do
    [[ -n "$changed_file" ]] || continue
    if [[ "$changed_file" == "$crate_dir/"* ]]; then
      return 0
    fi
  done < <(collect_changed_files)
  return 1
}

if [[ "$profile" == "smoke" ]]; then
  runs="${SHARO_FUZZ_SMOKE_RUNS:-20000}"
  max_total_time="${SHARO_FUZZ_SMOKE_MAX_TOTAL_TIME:-10}"
  max_len="${SHARO_FUZZ_SMOKE_MAX_LEN:-131072}"
else
  runs="${SHARO_FUZZ_FULL_RUNS:-100000}"
  max_total_time="${SHARO_FUZZ_FULL_MAX_TOTAL_TIME:-60}"
  max_len="${SHARO_FUZZ_FULL_MAX_LEN:-131072}"
fi

mapfile -t fuzz_crates < <(discover_fuzz_crates)
if [[ "${#fuzz_crates[@]}" -eq 0 ]]; then
  echo "check-fuzz: no fuzz crates discovered, skipping"
  exit 0
fi

ran_any=false
for crate_dir in "${fuzz_crates[@]}"; do
  if [[ "$mode" == "changed" ]] && ! should_run_crate_in_changed_mode "$crate_dir"; then
    continue
  fi
  mapfile -t targets < <(discover_fuzz_targets "$crate_dir")
  if [[ "${#targets[@]}" -eq 0 ]]; then
    continue
  fi
  for target in "${targets[@]}"; do
    echo "check-fuzz: crate=$crate_dir target=$target profile=$profile runs=$runs max_total_time=$max_total_time max_len=$max_len"
    (
      cd "$crate_dir"
      cargo fuzz run "$target" -- \
        -seed=1 \
        -runs="$runs" \
        -max_total_time="$max_total_time" \
        -max_len="$max_len"
    )
    ran_any=true
  done
done

if [[ "$ran_any" == false ]]; then
  if [[ "$mode" == "changed" ]]; then
    echo "check-fuzz: no fuzz-enabled crates changed, skipping"
  else
    echo "check-fuzz: no fuzz targets discovered, skipping"
  fi
  exit 0
fi

echo "check-fuzz: OK"
