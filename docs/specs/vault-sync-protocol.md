# Vault Sync Protocol

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Define tests before implementation work.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Property, and Integration tests.
> - Run `scripts/doc-lint.sh` before commit.

Updated: 2026-03-04
Status: active
Owner: sharo-core
Template-Profile: tdd-strict-v1

## Purpose

Define a deterministic, fail-closed sync protocol between non-canonical Knot vault notes and the canonical repository, without requiring direct vault CLI access. The protocol must preserve provenance, prevent protocol and memory drift, and provide auditable evidence for every sync action.

## Scope

### In Scope

- Explicit sync requests and staged execution (`pull`, `validate`, `promote`, optional `push-back`).
- Repo-local staging directory conventions for imported/exported note content.
- Manifest schema and evidence records required for sync operations.
- Required checks before staged content can be promoted to canonical repo paths.
- Minimal contract for agent-mediated vault operations (`get_note`, `list`, `replace_note`).

### Out of Scope

- Implicit or background mirroring between repo and vault.
- Bi-directional conflict resolution beyond explicit sync request scope.
- Vault-native authentication/session protocol details.
- Rich synchronization UX.

## Core Terms

- `Canonical Store`: this repository (`git main`) and tracked files.
- `External Store`: Knot vault and notes accessed through agent tools.
- `Sync Request`: explicit instruction containing source, target, direction, and scope.
- `Staging Workspace`: repo-local temp path for pulled or pending sync data.
- `Sync Manifest`: machine-readable record of files, hashes, directions, timestamps, and operation status.
- `Sync Evidence`: human-readable report that references one manifest and one sync request.
- `Promotion`: moving validated staged content into canonical repo paths.
- `Push-Back`: explicit repo-to-vault write after canonical verification.

## Interfaces / Contracts

- Sync request contract (must be explicit):
  - `source_path`
  - `target_path`
  - `direction` (`vault->repo` or `repo->vault`)
  - `scope` (list of paths/notes)
  - `intent` (`research`, `spec-sync`, `design-sync`, `other`)
- Stage contract:
  - `pull`: read external content into stage and produce manifest entries with content hash.
  - `validate`: run required repo checks against staged files and fail closed on error.
  - `promote`: copy from stage to canonical paths only if validation passed.
  - `push-back` (optional): write canonical repo content to vault only when explicitly requested.
  - `evidence`: append sync report with references to request and manifest.
- Manifest minimum fields per item:
  - `sync_id`
  - `direction`
  - `source_ref`
  - `staged_path`
  - `canonical_path`
  - `hash_sha256`
  - `status` (`pulled`, `validated`, `promoted`, `pushed_back`, `failed`)
  - `timestamp_utc`

## Invariants

- `I1 Canonical Authority`: canonical state is repo `main`; vault is never implicit source of truth.
- `I2 Explicit Scope`: every sync operation must match one explicit sync request.
- `I3 Staged-Only Import`: `vault->repo` content enters repo only through staging.
- `I4 Validation Gate`: no promotion or push-back is allowed before validation success.
- `I5 Hash Continuity`: promoted content hash must match validated staged hash.
- `I6 Fail Closed`: missing manifest fields or failed checks block promotion.
- `I7 Evidence Completeness`: every sync operation must leave both manifest and sync evidence.
- `I8 Non-Destructive External`: no vault write unless direction is explicitly `repo->vault`.

## Task Contracts

### Task 1: Pull Stage

**Preconditions**

- Explicit sync request exists with source, target, direction, and scope.
- Staging workspace path for `sync_id` exists or can be created.

**Invariants**

- `I1 Canonical Authority`
- `I2 Explicit Scope`
- `I3 Staged-Only Import`

**Postconditions**

- Staged files are present for each requested item.
- Manifest entries are created with required fields and `status=pulled`.

**Tests (must exist before implementation)**

Unit:
- `pull_manifest_requires_mandatory_fields`
- `pull_rejects_missing_scope`

Property:
- `pull_hash_is_stable_for_identical_content`

Integration:
- `pull_get_note_to_stage_creates_manifest_items`

### Task 2: Validate Stage

**Preconditions**

- Task 1 postconditions met.
- All staged files referenced by manifest exist.

**Invariants**

- `I4 Validation Gate`
- `I6 Fail Closed`

**Postconditions**

- Validation results are recorded per staged item.
- Any failed validation sets item status to `failed` and blocks promotion.

**Tests (must exist before implementation)**

Unit:
- `validate_blocks_on_missing_manifest_field`
- `validate_blocks_on_broken_links`

Property:
- `validate_is_idempotent_for_unchanged_stage`

Integration:
- `validate_runs_doc_lint_on_staged_docs_and_records_status`

### Task 3: Promote Stage

**Preconditions**

- Task 2 postconditions met with no failed items.
- Destination canonical paths are within repo scope.

**Invariants**

- `I1 Canonical Authority`
- `I4 Validation Gate`
- `I5 Hash Continuity`

**Postconditions**

- Canonical files are updated from stage.
- Manifest status for promoted items is `promoted`.

**Tests (must exist before implementation)**

Unit:
- `promote_rejects_unvalidated_item`
- `promote_rejects_hash_mismatch`

Property:
- `promote_preserves_content_hash`

Integration:
- `promote_stage_to_docs_updates_repo_files`

### Task 4: Push-Back Stage (Optional)

**Preconditions**

- Direction is explicitly `repo->vault`.
- Canonical source files are validated and current.

**Invariants**

- `I2 Explicit Scope`
- `I4 Validation Gate`
- `I8 Non-Destructive External`

**Postconditions**

- Vault update operations are executed only for requested scope.
- Manifest marks successful items `pushed_back`.

**Tests (must exist before implementation)**

Unit:
- `push_back_blocks_without_explicit_direction`
- `push_back_blocks_without_validated_source`

Property:
- `push_back_request_scope_is_closed_under_manifest`

Integration:
- `push_back_replace_note_updates_expected_note_ids_only`

### Task 5: Evidence and Audit

**Preconditions**

- At least one prior stage completed.

**Invariants**

- `I7 Evidence Completeness`

**Postconditions**

- One sync evidence note exists for the `sync_id`.
- Evidence references exact sync request and manifest paths.

**Tests (must exist before implementation)**

Unit:
- `evidence_requires_manifest_ref_and_sync_request_ref`

Property:
- `evidence_is_append_only_per_sync_id`

Integration:
- `sync_report_generation_matches_manifest_outcomes`

## Scenarios

1. Vault note import for research update:
- Request: `vault->repo`, scoped to one note.
- Result: note is staged, validated, and promoted to `docs/research/...` with manifest and evidence.
2. Repo spec publish to vault:
- Request: `repo->vault`, scoped to one canonical spec.
- Result: canonical file validated, then pushed to vault with evidence.
3. Mixed batch with one invalid staged file:
- Validation fails for one item.
- Promotion is blocked for the whole operation in MVP (all-or-nothing), with explicit failure records.

## Verification

- `scripts/doc-lint.sh --path docs/specs/vault-sync-protocol.md --strict-new`
- `scripts/doc-lint.sh --changed --strict-new`
- `scripts/check-sync-manifest.sh --path <manifest>`
- `scripts/sync-check.sh --dry-run --manifest <manifest>`

## Risks and Failure Modes

- Agent output mismatch with requested scope can cause accidental drift if scope checks are weak.
- Missing hash continuity checks can allow untracked content mutation between validation and promotion.
- Overly strict validation on non-canonical staged paths can block useful sync workflows.
- Partial push-back failure can leave repo and vault diverged unless evidence records include per-item status.

## Decisions (Resolved 2026-03-04)

- Promotion policy: all-or-nothing for MVP batches. Any failed item blocks promotion.
- Push-back retry policy: explicit/manual only in MVP. No automatic retries.
- Manifest schema policy: shell-level key and value checks are the MVP authority; JSON Schema is deferred.

## References

- [AGENTS.md](/home/dikini/Projects/sharo/AGENTS.md)
- [mvp.md](/home/dikini/Projects/sharo/docs/specs/mvp.md)
- [2026-03-04-doc-lint-gate-implementation-plan.md](/home/dikini/Projects/sharo/docs/plans/2026-03-04-doc-lint-gate-implementation-plan.md)
- [Sync Artifacts README](/home/dikini/Projects/sharo/docs/sync/README.md)
- [Sync Manifest Template](/home/dikini/Projects/sharo/docs/sync/sync-manifest.template.json)
- [Sync Evidence Template](/home/dikini/Projects/sharo/docs/sync/sync-evidence.template.md)
