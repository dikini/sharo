# Alias Resolution Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: add a minimal canonical alias registry and resolver script for known repo artifacts.
Architecture: store aliases in a simple TOML table and parse them with shell tooling to avoid dependencies; resolve aliases to absolute repo paths and fail closed on unknown/missing targets.
Tech Stack: bash, sed, awk, rg, existing policy scripts.
Template-Profile: tdd-strict-v1

---

### Task 1: Add alias registry

**Files:**

- Create: `docs/aliases.toml`
- Test: `scripts/alias-resolve.sh --list` (after script exists)

**Preconditions**

- [x] Canonical repo root is available via git.

**Invariants**

- [x] Alias keys remain stable and namespaced by artifact type.

**Postconditions**

- [x] Known artifacts are mapped to repo-relative paths.

**Tests (must exist before implementation)**

Unit:
- [x] `alias_registry_contains_spec_mvp`

Property:
- [x] `all_alias_targets_are_repo_relative`

Integration:
- [x] `resolver_list_shows_registered_aliases`

**Red Phase (required before code changes)**

Command: `test -f docs/aliases.toml`
Expected: fails because file does not exist.

**Implementation Steps**

1. Create `docs/aliases.toml` with `[aliases]` table.
2. Add known artifact mappings for specs, plans, templates, scripts, hooks, CI, sync artifacts.

**Green Phase (required)**

Command: `rg -n "^\"spec:mvp\"\s*=\s*\"docs/specs/mvp.md\"$" docs/aliases.toml`
Expected: one matching line.

**Refactor Phase (optional but controlled)**

Allowed scope: alias key names and ordering.
Re-run: `rg -n "^\"[a-z]+:" docs/aliases.toml`

**Completion Evidence**

- [x] Preconditions satisfied
- [x] Invariants preserved
- [x] Postconditions met
- [x] Unit, property, and integration tests passing
- [x] CHANGELOG.md updated

### Task 2: Add alias resolver script

**Files:**

- Create: `scripts/alias-resolve.sh`
- Modify: `CHANGELOG.md`
- Test: resolver CLI checks

**Preconditions**

- [x] Alias registry exists.

**Invariants**

- [x] Unknown aliases fail non-zero.
- [x] Output path is absolute and points to existing target.

**Postconditions**

- [x] Resolver supports `--list` and alias lookup.

**Tests (must exist before implementation)**

Unit:
- [x] `resolve_spec_mvp_returns_absolute_path`
- [x] `resolve_unknown_alias_fails`

Property:
- [x] `list_output_is_stable_and_sorted`

Integration:
- [x] `resolver_runs_without_external_dependencies`

**Red Phase (required before code changes)**

Command: `bash scripts/alias-resolve.sh @spec:mvp`
Expected: fails because script does not exist.

**Implementation Steps**

1. Implement argument parsing (`--list`, alias input, help).
2. Parse `[aliases]` entries from `docs/aliases.toml`.
3. Resolve to absolute path under repo root and verify file exists.
4. Make script executable and add changelog note.

**Green Phase (required)**

Command: `scripts/alias-resolve.sh @spec:mvp && scripts/alias-resolve.sh --list | head -n 5`
Expected: absolute path printed; list output present.

**Refactor Phase (optional but controlled)**

Allowed scope: error message text only.
Re-run: `scripts/alias-resolve.sh @unknown:alias || true`

**Completion Evidence**

- [x] Preconditions satisfied
- [x] Invariants preserved
- [x] Postconditions met
- [x] Unit, property, and integration tests passing
- [x] CHANGELOG.md updated
