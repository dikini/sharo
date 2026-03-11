# Conflict Resolution Policy

Updated: 2026-03-11
Status: active
Owner: platform
Template-Profile: tdd-strict-v1

## Instruction Priority

1. Reject unresolved conflict markers.
2. Enforce deterministic high-churn conflict-path allowlist rules.
3. Keep policy enforcement machine-checkable through one script entrypoint.

## Output Contract

- The policy defines explicit blocked patterns and allowed unmerged-file scope.
- Enforcement runs via `scripts/check-conflict-determinism.sh`.
- Violations fail with non-zero exit status.

## Model Compatibility Notes

- Policy language is deterministic and command-oriented for automated coding agents.
- File/path rules stay literal and unambiguous.

## Evidence / Verification Contract

- `scripts/check-conflict-determinism.sh`
- `scripts/run-shell-tests.sh --changed`

## Purpose

Define deterministic handling rules for high-churn merge-conflict files.

## Scope

### In Scope

- unresolved conflict marker rejection
- deterministic allowlist for conflict-prone files
- machine-checkable conflict policy enforcement

### Out of Scope

- hosted PR conflict UI workflows
- semantic merge automation beyond policy checks

## Task Contracts

### Task 1: Enforce deterministic conflict policy

**Preconditions**

- Repository is in a valid git working tree.

**Invariants**

- Unresolved conflict markers are always rejected.
- Unmerged paths outside allowlist are always rejected.

**Postconditions**

- Conflict-policy checks succeed only when repository state is deterministic per policy.

**Tests (must exist before implementation)**

Unit:
- `test_conflict_policy_detects_unresolved_markers`

Invariant:
- `test_conflict_policy_enforces_known_file_rules`

Integration:
- `test_conflict_policy_runs_in_fast_feedback_path`

## Rules

- Unresolved conflict markers (`<<<<<<<`, `=======`, `>>>>>>>`) are forbidden.
- If unmerged paths exist, only these files are policy-allowed:
  - `CHANGELOG.md`
  - `docs/tasks/tasks.csv`
  - `Cargo.lock`
- Any other unmerged path fails deterministic conflict policy checks.

## Enforcement

- `scripts/check-conflict-determinism.sh`
