# Agent Kernel Reasoning Implementation Implementation Plan

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Tests must be defined before implementation.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Property, and Integration tests.
> - Red phase before code changes; Green phase before completion.
> - Run `scripts/doc-lint.sh --changed --strict-new` before commit.

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: implement kernel/reasoning extraction and connector baseline in staged, behavior-preserving slices with explicit verification gates.
Architecture: first move task execution behind kernel/reasoning interfaces while preserving current scenario outputs; then add connector implementations behind the unified model connector port. Expand provider coverage incrementally from OpenAI-compatible paths to local and third-party adapters without changing daemon IPC/CLI contracts.
Tech Stack: Rust 2024, `serde`, `reqwest`, `tokio`, `clap`, existing sharo core/daemon store and scenario test suites.
Template-Profile: tdd-strict-v1

---

### Task 1: Extract Kernel/Reasoning Interfaces with Behavior Equivalence

Task-ID: TASK-KERNEL-IMPL-001

**Files:**

- Create: `crates/sharo-core/src/kernel.rs`
- Create: `crates/sharo-core/src/reasoning.rs`
- Modify: `crates/sharo-daemon/src/main.rs`
- Modify: `crates/sharo-daemon/src/store.rs`
- Test: `crates/sharo-daemon/tests/scenario_a.rs`, `crates/sharo-cli/tests/scenario_a_cli.rs`

**Preconditions**

- scenario A/B/C runtime behavior is passing on current mainline.
- daemon submit/approval flows are still directly coupled to store methods.

**Invariants**

- existing IPC request/response surfaces stay unchanged.
- scenario A/B/C external behavior remains equivalent.
- trace event sequence monotonicity and artifact provenance remain intact.

**Postconditions**

- daemon submit/approval operations are routed through kernel runtime interfaces.
- reasoning route decision is provided via explicit engine interface.

**Tests (must exist before implementation)**

Unit:
- `id_reasoning_engine_uses_connector_route_decision`

Property:
- `trace_event_sequence_is_monotonic`

Integration:
- `scenario_a_read_task_succeeds_with_verification_artifact`
- `scenario_b_pending_approval_survives_restart_and_can_be_resolved`
- `scenario_c_overlap_visibility_survives_restart`

**Red Phase (required before code changes)**

Command: `cargo test --workspace`
Expected: baseline passes before extraction changes; later targeted tests fail if interfaces are partially wired.

**Implementation Steps**

1. Add core kernel/reasoning interface modules and export them from `sharo-core`.
2. Add daemon kernel runtime adapter and route `submit-task`/`resolve-approval` through it.
3. Update store with route-decision injection helper while preserving old behavior values (`local_mock`) as default.
4. Re-run scenario suites to confirm no external behavior drift.

### Task 2: Add Unified Model Connector Baseline

Task-ID: TASK-KERNEL-IMPL-002

**Files:**

- Create: `crates/sharo-core/src/model_connector.rs`
- Create: `crates/sharo-core/src/model_connectors.rs`
- Modify: `crates/sharo-core/Cargo.toml`
- Test: `crates/sharo-core/tests/reasoning_connector_tests.rs`

**Preconditions**

- kernel/reasoning interfaces exist and daemon routing compiles.

**Invariants**

- connector contract exposes explicit timeout and error taxonomy.
- malformed provider responses do not map to successful empty outputs.

**Postconditions**

- deterministic connector supports parity mode.
- OpenAI-compatible connector and Ollama adapter exist behind unified contract.
- connector timeout profile and response-shape validation are enforced.

**Tests (must exist before implementation)**

Unit:
- `deterministic_connector_returns_provider_route_label`
- `extract_output_text_rejects_missing_text`

Property:
- `openai_compatible_connector_rejects_zero_timeout_profile`

Integration:
- `id_reasoning_engine_uses_connector_route_decision`
- `cargo test --workspace` remains green

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-core -- --nocapture`
Expected: connector-specific tests fail before connector implementations are added.

**Implementation Steps**

1. Implement connector profile/request/response types and error taxonomy.
2. Implement deterministic connector for parity mode.
3. Implement OpenAI-compatible `/v1/responses` connector.
4. Implement Ollama adapter delegating to the compatible connector path.
5. Enforce timeout and strict output text parsing.
6. Add connector contract tests and malformed-response tests.

### Task 3: Capture Completion Evidence and Governance Updates

Task-ID: TASK-KERNEL-IMPL-003

**Files:**

- Modify: `CHANGELOG.md`
- Modify: `docs/tasks/tasks.csv`
- Modify: `docs/plans/2026-03-05-agent-kernel-reasoning-implementation-plan.md`
- Test: `scripts/check-fast-feedback.sh`

**Preconditions**

- implementation tasks above are complete and tested.

**Invariants**

- changelog and task registry remain synchronized with actual code changes.

**Postconditions**

- changelog includes kernel/reasoning/connector additions and hardening notes.
- task registry includes implementation-plan tracking rows.

**Tests (must exist before implementation)**

Unit:
- `tasks_registry_entries_have_existing_sources`

Property:
- `tasks_sync_gate_passes_when_docs_plans_change`

Integration:
- `scripts/check-fast-feedback.sh` passes end-to-end

**Red Phase (required before code changes)**

Command: `scripts/check-tasks-sync.sh --changed`
Expected: fails until `docs/tasks/tasks.csv` is updated for new plan files.

**Implementation Steps**

1. Add changelog entries for kernel/reasoning/connector milestones.
2. Add/adjust task registry rows and statuses for design/implementation artifacts.
3. Execute full fast-feedback gate and preserve marker freshness.

**Green Phase (required)**

Command: `scripts/check-fast-feedback.sh`
Expected: all policy checks, tests, and matrix-map gate pass.

**Refactor Phase (optional but controlled)**

Allowed scope: connector internals and plan wording only; no IPC contract changes.
Re-run: `cargo test --workspace && scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated
