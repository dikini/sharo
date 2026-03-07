# <project-name>

One-sentence project summary: what this project does and for whom.

## Status

Choose one and update as needed:

- `prototype`
- `active development`
- `beta`
- `stable`
- `deprecated`

Stability note:

- Behavior, APIs, and persisted formats may change until this project is stable.

## Repository Layout

Document the top-level shape so contributors can orient quickly.

- `<path-or-crate-1>`: <purpose>
- `<path-or-crate-2>`: <purpose>
- `<path-or-crate-3>`: <purpose>

If this is a Rust workspace, prefer this pattern:

- `<workspace>-core`: shared domain logic and contracts
- `<workspace>-cli`: command-line entrypoint
- `<workspace>-daemon` (optional): long-running runtime/service

## Prerequisites

- `git`
- `<language toolchain and minimum version>`
- `<package manager / runtime dependencies>`

Rust example:

- Rust toolchain with `edition = 2024`
- `rust-version >= 1.93`
- `cargo nextest` (optional but recommended)

## Quick Start

```bash
<bootstrap-or-install-command>
<build-command>
<test-command>
```

Rust baseline:

```bash
scripts/bootstrap-dev.sh --check
cargo build --workspace
scripts/check-tests.sh
```

## Development Workflow

Document the mandatory feedback loop and pre-commit expectations.

Example policy:

1. Run fast feedback after each relevant edit batch:
   `scripts/check-fast-feedback.sh`
2. Keep `CHANGELOG.md` updated for task-completion work.
3. Follow commit message convention:
   `Conventional Commits` (<https://www.conventionalcommits.org/en/v1.0.0/>)
4. Install hooks once per clone:
   `scripts/install-hooks.sh`

## Verification Commands

List deterministic commands used by local and CI verification.

- `scripts/check-fast-feedback.sh`
- `scripts/check-tests.sh`
- `<project-specific checks>`

## Documentation

If this repo uses spec/plan governance, include explicit guidance.

- Canonical specs: `docs/specs/`
- Execution plans: `docs/plans/`
- Templates: `docs/templates/`

Recommended flow:

1. Update or create spec first.
2. Create/update plan.
3. Execute work against plan and record verification evidence.

## Contributing

- Open issues/PRs with clear problem statements and verification evidence.
- Keep changes scoped and reversible.
- Include tests for behavior changes.

## Security

- Report vulnerabilities via <security contact or process>.
- Do not commit secrets.
- Use scoped credentials and local environment files where applicable.

## License

<license-name-or-link>
