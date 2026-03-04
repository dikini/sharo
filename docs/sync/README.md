# Sync Artifacts

This directory defines canonical repo artifacts for explicit staged sync operations between the repository and non-canonical vault content.

## Purpose

- Standardize sync metadata and evidence.
- Keep sync operations auditable and reproducible.
- Enforce explicit scope and fail-closed promotion workflow.

## Files

- `sync-manifest.template.json`: template for machine-readable sync metadata.
- `sync-evidence.template.md`: template for human-readable sync evidence.
- `examples/valid.manifest.json`: runnable dry-run example manifest.

## Workflow

1. Create or receive an explicit sync request with source, target, direction, and scope.
2. Copy `sync-manifest.template.json` to a working manifest (for example `docs/sync/manifests/<sync-id>.json`).
3. Fill manifest entries during staged operations (`pull`, `validate`, `promote`, optional `push-back`).
4. Copy `sync-evidence.template.md` to an evidence note (for example `docs/sync/evidence/<sync-id>.md`).
5. Record command outcomes, hashes, and references to request and manifest.
6. Validate docs and manifest before completion.

## Commands

- Validate changed manifests:
  - `scripts/check-sync-manifest.sh --changed`
- Validate one manifest:
  - `scripts/check-sync-manifest.sh --path docs/sync/examples/valid.manifest.json`
- Execute staged protocol dry run:
  - `scripts/sync-check.sh --dry-run --manifest docs/sync/examples/valid.manifest.json`
- Run sync tool checks:
  - `scripts/tests/test-sync-tools.sh`

## Required References Per Sync

Each sync operation must provide:

- explicit sync request reference
- manifest path
- evidence path
- sync identifier (`sync_id`)

## Validation

- `scripts/doc-lint.sh --changed --strict-new`
- `scripts/check-sync-manifest.sh --changed`

## Notes

- Repository `main` is canonical.
- Vault sync is explicit only; no implicit mirroring.
- Vault writes (`repo->vault`) require explicit request direction.
