# Reasoning Context Fixed Point Loop Implementation Plan

Goal: Define and implement a policy-constrained reasoning context pipeline where context assembly converges through a bounded fixed-point loop and exposes explicit outward subsystem interfaces plus inward composition/filtering interfaces.
Architecture: Split reasoning into `resolve (I/O)` and `compose (pure)` phases. Resolve gathers typed component inputs (`system`, `persona`, `memory`, `runtime`, `goal`) from external subsystems using one uniform scope contract. Compose renders prompt context and runs policy fit/adjustment iterations until fit or bounded failure.
Tech Stack: Rust 1.93+, `sharo-core` traits + types, existing daemon/kernel/runtime store surfaces.
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-REASONING-FIT-DESIGN-001, TASK-REASONING-FIT-PLAN-001

## Intent

- Keep `goal` as one component, not the full prompt.
- Preserve deterministic, testable composition logic by isolating I/O.
- Make policy fit explicit: context must pass fit checks before model call.
- Keep component argument shape uniform, while allowing each resolver to use only needed fields.

## Design Decisions And Justification

1. Two-phase pipeline (`resolve -> compose`) is mandatory.
Reason: clear I/O boundaries, deterministic testing, and easier caching.

2. `TurnScope` contains immutable turn facts only.
Reason: derived artifacts (for example summaries) do not belong in scope; they belong in mutable context state created during resolution/composition.

3. Fixed-point loop controls policy fit.
Reason: single-shot truncation/filtering is fragile and opaque; iterative fit with explicit adjustments is auditable and safer.

4. Uniform resolver interface for `system`, `persona`, `memory`, `runtime`.
Reason: common orchestration path, consistent tracing, and pluggability.

5. Component-local and global filtering both exist.
Reason: local filtering enforces component invariants early; global fitting enforces end-to-end budget/policy constraints.

## Target Interfaces

### Outward Interfaces (subsystem boundaries)

- `SystemResolver`: static or mostly-static directives scoped by session/turn/task.
- `PersonaResolver`: role/behavior directives scoped by session/turn/task.
- `MemoryResolver`: retrieval over session/task/goal with relevance scoring and source provenance.
- `RuntimeContextResolver`: environment/policy/runtime capabilities and constraints.
- `SessionStorePort`: session/task metadata and state snapshots needed for resolution.
- `TaskPort`: current task and goal lineage state.
- `GoalUpdaterPort`: derives next-turn goal from prior result (separate from resolvers/composer).

### Inward Interfaces (reasoning internals)

- `TurnScope`: immutable request facts used by all resolvers.
- `ResolvedContext`: typed resolved outputs and provenance.
- `ContextState`: mutable assembled state for fit loop adjustments.
- `Composer`: pure rendering from `ContextState` to `PromptSpec`.
- `PolicyFitter`: validates rendered prompt against policy/budget/risk constraints.
- `AdjustmentPlanner`: creates deterministic, declarative multi-step adjustment programs when unfit.
- `AdjustmentApplier`: executes adjustment programs against `ContextState` and emits apply reports.

## Core Type Sketch (Rust)

```rust
pub struct TurnScope {
    pub session_id: String,
    pub task_id: String,
    pub turn_id: u64,
    pub goal: String,
}

pub trait ComponentResolver<O> {
    fn resolve(&self, scope: &TurnScope) -> Result<O, ResolveError>;
}

pub struct ResolvedContext {
    pub system: SystemContext,
    pub persona: PersonaContext,
    pub memory: MemoryContext,
    pub runtime: RuntimeContext,
    pub goal: GoalContext,
}

pub trait Composer {
    fn compose(&self, state: &ContextState) -> PromptSpec;
}

pub trait PolicyFitter {
    fn fit(&self, prompt: &PromptSpec, state: &ContextState) -> FitDecision;
}
```

### Adjustment Program Model

- `AdjustmentPlan` is a collection, not a single operation.
- `AdjustmentPlan` contains ordered `AdjustmentStep` items with explicit rationale.
- `AdjustmentStep` is declarative and component-targeted (memory/persona/runtime/system shaping).

Suggested structure:

```rust
pub struct AdjustmentPlan {
    pub plan_id: String,
    pub rationale: String,
    pub steps: Vec<AdjustmentStep>,
}

pub enum AdjustmentStep {
    DropMemoryByRank { max_items: usize },
    CompressMemoryToTokens { token_budget: usize },
    RedactRuntimeFields { fields: Vec<String> },
    ClampPersonaVerbosity { level: String },
}
```

Execution semantics:

- deterministic step order (as listed in `steps`)
- either atomic apply (preferred) or stepwise apply with explicit partial-failure policy
- always emit `ApplyReport` with before/after state hashes, changed components, and failure reason if any
- treat repeated no-op plans as non-progress and fail fast

This keeps `PolicyFitter` responsible for deciding *what* to change and `AdjustmentApplier` for *how* to apply changes predictably.

## Fixed-Point Loop

```rust
state = ContextState::from_resolved(resolve_all(scope)?);
for i in 0..MAX_FIT_ITERS {
    let prompt = composer.compose(&state);
    match fitter.fit(&prompt, &state) {
        FitDecision::Fitted => return Ok((state, prompt)),
        FitDecision::Adjust(plan) => {
            let apply_report = applier.apply(&mut state, &plan)?;
            trace_iteration(i, &plan, &apply_report);
        }
    }
}
Err(ReasoningError::ContextPolicyFitFailed)
```

Loop constraints:

- bounded by `MAX_FIT_ITERS`
- deterministic adjustment ordering
- monotonic progress requirement (no repeated identical state hash)
- trace each iteration decision for auditability
- adjustment plan is a first-class trace artifact

## Filtering Strategy

1. Component-local filtering during `resolve`
- Examples: dedupe memory hits, redact disallowed runtime fields, enforce persona schema validity.

2. Global fit filtering in loop
- Examples: token budget trimming, priority-based memory compression, strict policy redaction.
 - Implemented as ordered `AdjustmentPlan.steps` to avoid ad-hoc one-off mutations.

3. Stop conditions
- fit achieved
- budget exhausted with explicit failure reason
- non-progress cycle detected

## Provider Rendering Contract

- `PromptSpec` is provider-neutral.
- OpenAI-compatible adapter maps `PromptSpec` to provider payload (`/v1/responses` shape).
- Other providers (Anthropic, OpenRouter variants, local runtimes) adapt from same `PromptSpec`.

This preserves one composition engine with multiple rendering adapters.

## Note: Future Tool Calling Integration

- Tool call results are modeled as part of `runtime` context (for example `runtime.tool_outputs`).
- Tool outputs are never injected directly into prompts without passing through component-local filtering and global fit-loop policy checks.
- Future tool orchestration should use the same resolve/compose/fix-point pipeline so model, memory, and tool-derived context share one policy boundary and one trace model.

---

### Task 1: Define Core Reasoning Context And Loop Interfaces

**Files:**

- Create: `crates/sharo-core/src/reasoning_context.rs`
- Modify: `crates/sharo-core/src/lib.rs`, `crates/sharo-core/src/reasoning.rs`
- Test: `crates/sharo-core/tests/reasoning_context_tests.rs`

**Preconditions**

- Existing kernel/reasoning path compiles and passes tests.

**Invariants**

- Existing connector request/response behavior remains unchanged until wiring task.
- `TurnScope` contains only immutable turn facts.

**Postconditions**

- New context/fit interfaces compile and are callable from reasoning module without behavior change.

**Tests (must exist before implementation)**

Unit:
- `turn_scope_excludes_derived_fields`
- `fit_loop_stops_on_fitted_or_max_iters`

Property:
- `fit_loop_state_hash_progress_is_monotonic_or_fails`

Integration:
- `id_reasoning_engine_compatibility_with_context_defaults`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-core reasoning_context_tests -- --nocapture`
Expected: test targets fail because interfaces are missing.

**Implementation Steps**

1. Add typed context/loop interfaces and minimal in-memory implementations.
2. Add no-op/default fitter+composer path preserving current one-turn behavior.

**Green Phase (required)**

Command: `cargo test -p sharo-core reasoning_context_tests -- --nocapture`
Expected: all new tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: `crates/sharo-core/src/reasoning*.rs`
Re-run: `cargo test -p sharo-core`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated

---

### Task 2: Implement Outward Resolver Ports And Filtering Hooks

**Files:**

- Create: `crates/sharo-core/src/context_resolvers.rs`
- Modify: `crates/sharo-core/src/reasoning.rs`, `crates/sharo-daemon/src/kernel.rs`
- Test: `crates/sharo-core/tests/context_resolver_tests.rs`

**Preconditions**

- Task 1 interfaces exist and compile.

**Invariants**

- Resolver contract signature is uniform across components.
- Resolver outputs include provenance metadata.

**Postconditions**

- Reasoning path can fetch component contexts through resolvers and produce `ResolvedContext`.

**Tests (must exist before implementation)**

Unit:
- `resolver_contract_is_uniform_for_all_components`
- `component_local_filtering_applies_before_compose`

Property:
- `resolver_output_order_is_deterministic`

Integration:
- `kernel_submit_uses_resolved_context_before_model_call`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-core context_resolver_tests -- --nocapture`
Expected: tests fail due to missing resolver wiring.

**Implementation Steps**

1. Add resolver trait implementations/stubs for system/persona/memory/runtime.
2. Wire reasoning to call resolvers before compose loop.

**Green Phase (required)**

Command: `cargo test -p sharo-core context_resolver_tests -- --nocapture`
Expected: all tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: resolver modules only
Re-run: `cargo test -p sharo-core`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated

---

### Task 3: Wire Fixed-Point Fit Loop To Runtime Persistence And CLI Visibility

**Files:**

- Modify: `crates/sharo-core/src/reasoning.rs`, `crates/sharo-daemon/src/store.rs`, `crates/sharo-cli/src/main.rs`
- Test: `crates/sharo-daemon/tests/scenario_a.rs`, `crates/sharo-cli/tests/scenario_a_cli.rs`

**Preconditions**

- Task 1 and Task 2 completed.

**Invariants**

- Every fit-loop iteration is traceable.
- Failure to fit returns explicit machine-parseable reason.

**Postconditions**

- Runtime records include fit-loop outcomes and selected filtering decisions.
- CLI surfaces loop outcomes in trace/artifact outputs.

**Tests (must exist before implementation)**

Unit:
- `fit_loop_records_adjustment_events`

Property:
- `loop_terminates_within_max_iters`

Integration:
- `trace_and_artifacts_expose_fit_loop_decisions`

**Red Phase (required before code changes)**

Command: `cargo test -p sharo-daemon scenario_a -- --nocapture`
Expected: assertions fail before loop outcome persistence exists.

**Implementation Steps**

1. Persist fit-loop decision events and adjustment artifacts.
2. Extend CLI surfaces for loop decision inspection.

**Green Phase (required)**

Command: `cargo test --workspace`
Expected: all tests pass.

**Refactor Phase (optional but controlled)**

Allowed scope: reasoning/store/cli display formatting
Re-run: `scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, property, and integration tests passing
- CHANGELOG.md updated
