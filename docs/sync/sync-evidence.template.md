# Sync Evidence: <sync_id>

Updated: <YYYY-MM-DD>
Owner: <team/person>
Status: draft | completed | failed

## Request Reference

- Sync request: `<path-or-link>`
- Direction: `vault->repo` | `repo->vault`
- Scope summary: `<brief scope>`

## Manifest Reference

- Manifest path: `<path>`
- Manifest id: `<sync_id>`

## Stage Outcomes

| Stage | Status | Notes |
|---|---|---|
| Pull | pending | |
| Validate | pending | |
| Promote | pending | |
| Push-Back (optional) | n/a | |

## Item Outcomes

| Item ID | Source Ref | Canonical Path | Hash | Final Status | Notes |
|---|---|---|---|---|---|
| `<item-id>` | `<source-ref>` | `<canonical-path>` | `<sha256>` | `pulled` | |

## Commands and Checks

- `scripts/doc-lint.sh --changed --strict-new`
- `scripts/check-sync-manifest.sh --path <manifest>` (planned)

## Evidence Summary

- Preconditions met: yes | no
- Validation gate passed: yes | no
- Hash continuity verified: yes | no
- Promotion performed: yes | no
- Push-back performed: yes | no

## References

- [Vault Sync Protocol](/home/dikini/Projects/sharo/docs/specs/vault-sync-protocol.md)
- [Sync Artifacts README](/home/dikini/Projects/sharo/docs/sync/README.md)
