# Connector Blocking Execution

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Define tests before implementation work.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Property, and Integration tests.
> - Run `scripts/doc-lint.sh` before commit.

Updated: 2026-03-05
Status: active
Owner: runtime
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-CONNECTOR-POOL-SPEC-001, TASK-CONNECTOR-POOL-PLAN-001

## Purpose

Define the daemon-side execution model for blocking model connectors so runtime behavior is safe under async Tokio execution and scales predictably under load.

## Scope

### In Scope

- Bounded connector worker execution for blocking HTTP connectors (`openai`, `ollama`).
- Explicit queue/backpressure behavior when connector capacity is exhausted.
- Configured pool policy with fixed-size baseline and dynamic scaling policy fields.
- Observable metrics and failure surfaces for overload and timeout behavior.

### Out of Scope

- Replacing existing connector protocol semantics.
- New provider protocols beyond current OpenAI-compatible and Ollama paths.
- Distributed or multi-process connector execution.

## Core Terms

- `BlockingConnectorExecutor`: runtime component that executes connector turns off async request threads.
- `ConnectorWorkerPool`: bounded worker pool used by daemon kernel for blocking connector calls.
- `PoolPolicy`: policy describing min/max workers, queue capacity, and scale behavior.
- `OverloadReject`: explicit failure when queue capacity is exhausted.
- `ScaleController`: controller that adjusts worker count based on policy and load signals.

## Interfaces / Contracts

- `ConnectorWorkerPool::submit(job) -> Result<ModelTurnResponse, ConnectorError>`.
- `ConnectorExecutionPolicy` from daemon TOML with fields:
  - `min_threads`
  - `max_threads`
  - `queue_capacity`
  - `scale_up_queue_threshold`
  - `scale_down_idle_ms`
  - `cooldown_ms`
- Runtime behavior contract:
  - deterministic connector may run inline.
  - blocking connectors must execute through `ConnectorWorkerPool`.
  - queue full must return explicit machine-parseable overload error.

## Invariants

- No per-request unbounded OS thread creation in connector execution path.
- Per-task/session ordering semantics remain unchanged.
- Connector timeout behavior remains enforced by model profile.
- Connector failures never produce false success task outcomes.
- Pool scale decisions never exceed configured `[min_threads, max_threads]`.

## Task Contracts

### Task 1: Bounded Blocking Executor Baseline

**Preconditions**

- Daemon kernel routes OpenAI/Ollama calls through connector adapter.

**Invariants**

- No async-runtime panic from blocking connector lifecycle.
- No unbounded thread spawn per request.

**Postconditions**

- Blocking connectors execute via bounded pool with explicit queue capacity.

**Tests (must exist before implementation)**

Unit:
- `bounded_pool_reuses_workers_without_per_request_spawn`

Property:
- `pool_worker_count_never_exceeds_max_threads`

Integration:
- `openai_turn_succeeds_with_content_visible_in_trace_and_artifacts`
- `queue_overflow_returns_overload_error`

### Task 2: Adaptive Scaling Policy

**Preconditions**

- Task 1 baseline pool exists and is used for blocking connectors.

**Invariants**

- Scale controller obeys min/max bounds and cooldown constraints.
- Scale-down never terminates in-flight work.

**Postconditions**

- Pool size can grow/shrink with configured policy.
- Scaling decisions are observable via metrics or trace logs.

**Tests (must exist before implementation)**

Unit:
- `scale_controller_scales_up_on_queue_pressure`
- `scale_controller_scales_down_after_idle_window`

Property:
- `scale_transitions_remain_within_policy_bounds`

Integration:
- `burst_load_scales_up_and_recovers_to_idle_min`
- `ordering_invariants_hold_under_scaling`

## Scenarios

- S1: single OpenAI turn returns content and keeps daemon stable.
- S2: burst submissions up to queue and worker limits complete without extra thread spawn.
- S3: overload beyond queue capacity returns explicit error, no daemon crash.
- S4: sustained pressure triggers scale-up (future phase) and idle period triggers scale-down.

## Verification

- `cargo test -p sharo-daemon --test scenario_a`
- `cargo test -p sharo-core --test reasoning_connector_tests`
- targeted pool tests for bounded worker and scaling policy
- `scripts/check-fast-feedback.sh`

## Risks and Failure Modes

- Queue starvation or head-of-line blocking for long connector calls.
- Misconfigured policy (`max_threads < min_threads`, zero queue).
- Burst load causing timeouts before scale-up reacts.
- Metric visibility gaps hiding saturation conditions.

## Open Questions

- Should overload return `Unavailable` vs dedicated `Overloaded` connector error kind?
- Should deterministic connector optionally use same pool for uniform observability?
- What default policy values are safe for low-resource developer machines?

## References

- [Reasoning Context Fixed Point Loop](/home/dikini/Projects/sharo/docs/plans/2026-03-05-reasoning-context-fixed-point-loop-plan.md)
- [Agent Kernel Reasoning Implementation Plan](/home/dikini/Projects/sharo/docs/plans/2026-03-05-agent-kernel-reasoning-implementation-plan.md)
