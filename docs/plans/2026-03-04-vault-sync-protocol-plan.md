# Vault Sync Protocol Implementation Plan

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Property, and Integration tests.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: implement a minimal staged sync toolchain that enforces explicit repo-vault sync scope, validation gates, and audit evidence.
Architecture: add small shell scripts around a manifest-first workflow (`pull -> validate -> promote -> optional push-back`), keep all checks repo-local, and require explicit records for all external operations. Integrate with existing doc lint and policy hooks rather than creating a separate policy path.
Tech Stack: bash, git, `rg`, `awk`, existing scripts in `scripts/`, markdown docs in `docs/`.
Template-Profile: tdd-strict-v1

---

### Task 1: Define Manifest and Sync Evidence Artifacts

**Files:**

- Create: `docs/sync/README.md`
- Create: `docs/sync/sync-manifest.template.json`
- Create: `docs/sync/sync-evidence.template.md`
- Modify: `docs/specs/vault-sync-protocol.md`
- Test: `scripts/doc-lint.sh` (existing)

**Preconditions**

- [x] `docs/specs/vault-sync-protocol.md` exists and is active.
- [x] Canonical source policy in `AGENTS.md` remains unchanged.

**Invariants**

- [x] Manifest field names must match the spec.
- [x] Evidence file must reference both request and manifest.
- [x] No vault access is required for this task.

**Postconditions**

- [x] Reusable templates exist for manifest and evidence files.
- [x] Artifact usage instructions are documented in `docs/sync/README.md`.

**Tests (must exist before implementation)**

Unit:
- [x] `manifest_template_contains_required_keys`

Property:
- [x] `template_roundtrip_preserves_required_key_set`

Integration:
- [x] `doc_lint_passes_for_new_sync_docs`

**Red Phase (required before code changes)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: baseline pass before adding new sync docs; later fails if required sections are missing.

**Implementation Steps**

1. Add `docs/sync/README.md` with staged workflow and artifact lifecycle.
2. Add JSON manifest template with required fields per item and sync-level metadata.
3. Add markdown evidence template with explicit request ref, manifest ref, and outcome table.
4. Align references in `docs/specs/vault-sync-protocol.md` if needed.

**Green Phase (required)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: docs lint passes with new sync templates and spec references.

**Refactor Phase (optional but controlled)**

Allowed scope: wording only in `docs/sync/*` and spec references.
Re-run: `scripts/doc-lint.sh --changed --strict-new`

**Completion Evidence**

- [x] Preconditions satisfied
- [x] Invariants preserved
- [x] Postconditions met
- [x] Unit, property, and integration tests passing
- [x] CHANGELOG.md updated

### Task 2: Add Manifest Validator Script

**Files:**

- Create: `scripts/check-sync-manifest.sh`
- Modify: `scripts/doc-lint.sh`
- Modify: `.githooks/pre-commit`
- Modify: `.github/workflows/policy-checks.yml`
- Test: `tests/scripts/check-sync-manifest/*.sh` (or shell fixtures under `scripts/tests/`)

**Preconditions**

- [x] Manifest template exists from Task 1.
- [x] Script execution style matches existing shell tooling.

**Invariants**

- [x] Missing required fields must fail closed.
- [x] Validator must not call external vault APIs.
- [x] Validation behavior must be deterministic on the same input.

**Postconditions**

- [x] Manifest checker exits non-zero on malformed manifests.
- [x] Pre-commit and CI run the checker when sync artifacts are changed.

**Tests (must exist before implementation)**

Unit:
- [x] `check_sync_manifest_fails_missing_sync_id`
- [x] `check_sync_manifest_fails_missing_hash`

Property:
- [x] `check_sync_manifest_is_idempotent`

Integration:
- [x] `policy_checks_runs_manifest_validation_on_changed_sync_artifacts`

**Red Phase (required before code changes)**

Command: `bash scripts/check-sync-manifest.sh docs/sync/sync-manifest.template.json`
Expected: fails before script exists; later fails on intentionally broken fixture.

**Implementation Steps**

1. Implement `scripts/check-sync-manifest.sh` with required field and status validation.
2. Add optional `--path` and default path behavior consistent with other scripts.
3. Hook the checker into pre-commit when `docs/sync/*` or manifest files are staged.
4. Hook the checker into CI policy workflow.

**Green Phase (required)**

Command: `scripts/check-sync-manifest.sh --path docs/sync/sync-manifest.template.json && scripts/doc-lint.sh --changed --strict-new`
Expected: manifest checker and doc lint both pass.

**Refactor Phase (optional but controlled)**

Allowed scope: script help text and diagnostics only.
Re-run: `scripts/check-sync-manifest.sh --path docs/sync/sync-manifest.template.json`

**Completion Evidence**

- [x] Preconditions satisfied
- [x] Invariants preserved
- [x] Postconditions met
- [x] Unit, property, and integration tests passing
- [x] CHANGELOG.md updated

### Task 3: Add Staged Sync Runner (Repo-Local)

**Files:**

- Create: `scripts/sync-check.sh`
- Create: `docs/sync/examples/`
- Modify: `docs/sync/README.md`
- Test: shell fixtures for dry-run and failure paths

**Preconditions**

- [x] Manifest validator exists and passes.
- [x] Sync spec invariants are approved.

**Invariants**

- [x] Runner must enforce stage order (`pull -> validate -> promote`).
- [x] Runner must block `push-back` unless explicit direction is `repo->vault`.
- [x] Runner must never mutate canonical files on `--dry-run`.

**Postconditions**

- [x] Runner supports dry-run with deterministic output.
- [x] Runner records per-item status transitions in manifest/evidence files.

**Tests (must exist before implementation)**

Unit:
- [x] `sync_check_rejects_invalid_stage_order`
- [x] `sync_check_blocks_push_back_without_direction`

Property:
- [x] `dry_run_is_non_mutating`

Integration:
- [x] `sync_check_dry_run_with_example_manifest_reports_expected_transitions`

**Red Phase (required before code changes)**

Command: `bash scripts/sync-check.sh --dry-run --manifest docs/sync/examples/valid.manifest.json`
Expected: fails before script exists.

**Implementation Steps**

1. Implement `scripts/sync-check.sh` stage orchestration with dry-run mode first.
2. Add manifest transition updates with status checks.
3. Add example manifests and evidence output samples.
4. Document usage in `docs/sync/README.md`.

**Green Phase (required)**

Command: `scripts/sync-check.sh --dry-run --manifest docs/sync/examples/valid.manifest.json`
Expected: command exits `0` and prints expected stage transitions without repo writes.

**Refactor Phase (optional but controlled)**

Allowed scope: output formatting and error messages only.
Re-run: `scripts/sync-check.sh --dry-run --manifest docs/sync/examples/valid.manifest.json`

**Completion Evidence**

- [x] Preconditions satisfied
- [x] Invariants preserved
- [x] Postconditions met
- [x] Unit, property, and integration tests passing
- [x] CHANGELOG.md updated

### Task 4: Policy Integration and MVP Cross-References

**Files:**

- Modify: `AGENTS.md`
- Modify: `docs/specs/mvp.md`
- Modify: `.github/workflows/policy-checks.yml`
- Modify: `CHANGELOG.md`
- Test: `scripts/doc-lint.sh`, policy workflow local command subset

**Preconditions**

- [x] Tasks 1-3 completed.
- [x] Existing policy hooks are passing.

**Invariants**

- [x] Canonical source of truth remains repo `main`.
- [x] External sync remains explicit and non-implicit.
- [x] Policy checks remain lightweight and shell-first.

**Postconditions**

- [x] Governance and MVP docs reference the sync protocol and runner.
- [x] Policy checks include sync-specific validation in CI and hooks.

**Tests (must exist before implementation)**

Unit:
- [x] `agents_policy_section_mentions_sync_manifest_and_evidence`

Property:
- [x] `policy_checks_do_not_depend_on_external_vault_access`

Integration:
- [x] `full_policy_check_path_passes_with_valid_sync_artifacts`

**Red Phase (required before code changes)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: baseline pass before cross-reference updates; then fail if stale links are introduced.

**Implementation Steps**

1. Add AGENTS policy guidance for staged sync protocol usage.
2. Add reference from MVP spec to vault sync protocol where external knowledge sync is discussed.
3. Extend policy workflow to run sync checks in applicable paths.
4. Update changelog entries for all sync protocol additions.

**Green Phase (required)**

Command: `scripts/doc-lint.sh --changed --strict-new && scripts/check-rust-policy.sh`
Expected: all policy checks pass; rust check skips or passes depending on project state.

**Refactor Phase (optional but controlled)**

Allowed scope: wording and reference paths only.
Re-run: `scripts/doc-lint.sh --changed --strict-new`

**Completion Evidence**

- [x] Preconditions satisfied
- [x] Invariants preserved
- [x] Postconditions met
- [x] Unit, property, and integration tests passing
- [x] CHANGELOG.md updated
