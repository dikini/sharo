# OpenAI Live Smoke Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: add a repeatable live smoke scenario that proves OpenAI connectivity and basic response-shape compliance while surfacing answer content directly in CLI-visible outputs.
Architecture: add one operator-facing shell script that boots daemon+IPC, submits a task, then extracts model content from trace/artifact surfaces; pair it with Bats tests that verify guardrails and deterministic dry smoke behavior. Keep this separate from unit/integration Rust tests so live credentials stay opt-in.
Tech Stack: Bash, Bats, existing `sharo-daemon` + `sharo` CLI IPC commands.
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-LIVE-OPENAI-SMOKE-001

---

### Task 1: Implement Live OpenAI Smoke Script With Clear Content Surface

**Files:**

- Create: `scripts/openai-live-smoke.sh`
- Create: `scripts/tests/test-openai-live-smoke.bats`
- Modify: `docs/tasks/tasks.csv`, `CHANGELOG.md`
- Test: `scripts/tests/test-openai-live-smoke.bats`

**Preconditions**

- `sharo-daemon` and `sharo` IPC flow is operational.
- Config defaults to `~/.config/sharo/daemon.toml`.

**Invariants**

- Script never prints secret auth values.
- Script fails closed when config or auth preconditions are missing.
- Script reports content from runtime trace/artifacts, not internal daemon logs.

**Postconditions**

- Operators can run one command to validate OpenAI connectivity.
- Output includes non-empty model content from both trace and artifact paths.
- Shell tests cover validation and deterministic non-network success path.

**Tests (must exist before implementation)**

Unit:
- `openai_live_smoke_help_succeeds`
- `openai_live_smoke_requires_auth_env_when_openai`

Property:
- `openai_live_smoke_parses_task_id_from_submit_output`

Integration:
- `openai_live_smoke_deterministic_mode_surfaces_model_content`

**Red Phase (required before code changes)**

Command: `scripts/install-bats.sh >/dev/null && "$(scripts/install-bats.sh)" scripts/tests/test-openai-live-smoke.bats`
Expected: failing tests because script is missing.

**Implementation Steps**

1. Add Bats tests that define expected script behavior and failure modes.
2. Implement `scripts/openai-live-smoke.sh` with config/auth guards, daemon lifecycle, and content extraction.
3. Update task registry and changelog for this new verification surface.

**Green Phase (required)**

Command: `scripts/install-bats.sh >/dev/null && "$(scripts/install-bats.sh)" scripts/tests/test-openai-live-smoke.bats`
Expected: all task tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `scripts/openai-live-smoke.sh`, `scripts/tests/test-openai-live-smoke.bats`
Re-run: `scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated
