# Sharo MVP Spec Set

## 1. Document Purpose

This document is the consolidated MVP spec set for Sharo's absolute minimal functional prototype. It translates the current architecture and subsystem notes into one decision-complete specification for a thin but real system where every major part of the architecture exists, even if the first implementation is bare-bones.

The MVP posture is locked:

- all architecture parts are present
- CLI is the first operator surface
- the runtime is a thin vertical slice, not a feature-rich assistant
- enrichment is ordered risk-first
- additions after MVP must be research-grounded or justified by operational evidence

This document is intentionally specification-oriented rather than implementation-oriented. It defines required behavior, interfaces, state, invariants, and verification rather than code layout.

## 2. MVP Summary

The MVP is a single-daemon assistant runtime with one CLI client surface and one end-to-end execution path. It supports multiple concurrent sessions, task submission, a one-step-at-a-time kernel, explicit model routing, explicit policy evaluation, execution of one manifest-backed capability, durable trace and artifact persistence, minimal coordination records, and recovery across restart.

The MVP must prove three things:

1. a read-oriented task can succeed end to end through every major subsystem
2. a restricted task can be blocked or approval-gated without hidden execution
3. overlap between concurrent sessions can become durable, inspectable runtime state without requiring rich built-in arbitration

The MVP is not expected to provide broad autonomy, rich UX, or many capabilities. It is expected to demonstrate that the architecture is real, bounded, auditable, and internally coherent.

### In Scope

- one daemon process
- one CLI client
- multiple concurrent sessions through the daemon
- one local persistence layer
- one local or mocked model path
- one explicit route decision artifact per model-assisted step
- one read-only capability
- one restricted capability path
- one policy allow path
- one policy block or approval path
- exact trace persistence
- artifact persistence with provenance
- durable policy, approval, and coordination records
- minimal observability and cleanup hooks through traces, artifacts, and verification records
- recovery after restart

### Out of Scope

- full TUI
- full Knot-native UI
- broad MCP support
- plugin hot reload
- autonomous long-running task execution
- multi-agent planning
- rich conflict resolution or arbitration
- advanced retrieval ranking
- large capability catalogs
- broad cloud routing

## 3. Global Invariants

These invariants apply across the whole MVP and must be restated or referenced by every subsystem section.

### Runtime Invariants

- Every `Task` has exactly one durable lifecycle state at a time.
- Every meaningful runtime transition emits a `TraceEvent`.
- Every `Step` ends as `completed`, `failed`, `blocked`, or `awaiting_approval`.
- No runtime-critical state is represented only in model-generated narration.
- Detected overlap between sessions, tasks, or resource claims must become durable runtime state rather than implicit operator knowledge.

### Control Invariants

- The kernel advances one step at a time.
- A capability executes only through a manifest-backed contract.
- Policy is evaluated before any restricted or side-effecting step executes.
- Model routing is explicit, recorded, and inspectable.

### Memory Invariants

- Exact trace and artifact records persist before any derived memory.
- Every `Artifact` can be traced back to the exact `Trace` chain that produced it.
- Derived memory never deletes or overwrites exact provenance.
- Bindings may remain opaque while still having a visible handle, lifecycle, and owner.
- `PolicyDecision`, `Approval`, and coordination records are exact records, not optional annotations.

### Safety Invariants

- Denied steps do not execute.
- Approval-gated steps do not proceed without a durable `Approval` record.
- The system does not require raw sensitive values in model-visible text in order to function.
- Capability scope cannot exceed manifest-declared authority.

### Operator Invariants

- The CLI can inspect task state, blocking reason, and final outcome.
- The operator can inspect any recorded coordination or overlap state through normal task, trace, or artifact retrieval.
- Restart recovery restores from durable state, not reconstructed narration.
- Failed and blocked tasks remain inspectable after completion.

## 4. Core Types and State Machine Spec

This section defines the public logical types used by the MVP. Exact field names may vary at implementation time, but the semantic fields and transitions are fixed by this spec.

### Core Types

#### `Session`

Required fields:

- `session_id`
- `created_at`
- `updated_at`
- `origin_surface`
- `status`
- `visibility_scope`
- `policy_profile_version`
- `personality_profile_version`

#### `Task`

Required fields:

- `task_id`
- `session_id`
- `goal`
- `task_state`
- `autonomy_mode`
- `budget`
- `created_at`
- `updated_at`
- `current_step_id`
- `result_artifact_id`
- `last_trace_event_id`

#### `TaskState`

Required states:

- `submitted`
- `queued`
- `running`
- `awaiting_approval`
- `blocked`
- `succeeded`
- `failed`
- `cancelled`

#### `Step`

Required fields:

- `step_id`
- `task_id`
- `step_kind`
- `step_state`
- `summary`
- `preconditions`
- `postconditions`
- `requested_capability`
- `requested_route`
- `created_at`
- `updated_at`

#### `StepState`

Required states:

- `proposed`
- `ready`
- `executing`
- `awaiting_approval`
- `blocked`
- `completed`
- `failed`

#### `Capability`

Required fields:

- `capability_id`
- `name`
- `manifest_version`
- `trust_state`
- `side_effect_class`
- `scope`
- `executor_type`

#### `CapabilityManifest`

Required fields:

- `name`
- `version`
- `description`
- `executor_type`
- `input_schema`
- `output_contract`
- `side_effect_class`
- `scope`
- `timeout_ms`
- `cancellable`
- `trust_state`

#### `Binding`

Required fields:

- `binding_id`
- `task_id`
- `name`
- `visibility`
- `value_kind`
- `backing_ref`
- `created_at`
- `updated_at`
- `status`

#### `BindingVisibility`

Required values:

- `model_visible`
- `engine_only`
- `approval_gated`

#### `Artifact`

Required fields:

- `artifact_id`
- `task_id`
- `artifact_kind`
- `summary`
- `content_ref`
- `produced_by_step_id`
- `produced_by_trace_event_id`
- `created_at`

#### `ArtifactKind`

Required values for MVP:

- `route_decision`
- `capability_result`
- `verification_result`
- `failure_record`
- `final_result`

#### `Trace`

Required fields:

- `trace_id`
- `task_id`
- `session_id`
- `event_sequence`
- `events`
- `created_at`
- `updated_at`

#### `TraceEvent`

Required fields:

- `event_id`
- `trace_id`
- `task_id`
- `step_id`
- `event_type`
- `timestamp`
- `payload_ref`
- `correlation_id`

#### `PolicyProfile`

Required fields:

- `version`
- `autonomy_mode_defaults`
- `approval_thresholds`
- `allowed_capability_classes`
- `routing_constraints`

#### `PersonalityProfile`

Required fields:

- `version`
- `voice_profile`
- `initiative_level`
- `relationship_posture`
- `risk_posture`

#### `PolicyDecision`

Required fields:

- `decision_id`
- `task_id`
- `step_id`
- `decision`
- `rationale`
- `constraints`
- `policy_profile_version`
- `created_at`

Required decision values:

- `allow`
- `deny`
- `require_approval`

#### `Approval`

Required fields:

- `approval_id`
- `task_id`
- `step_id`
- `approval_state`
- `requested_at`
- `expires_at`
- `resolved_at`
- `resolver_id`
- `resolution_note`

#### `ApprovalState`

Required values:

- `pending`
- `approved`
- `denied`
- `expired`

#### `ModelProfile`

Required fields:

- `model_profile_id`
- `name`
- `provider_kind`
- `privacy_class`
- `strength_class`
- `latency_class`
- `cost_class`
- `available`

#### `RouteDecision`

Required fields:

- `route_decision_id`
- `task_id`
- `step_id`
- `model_profile_id`
- `rationale`
- `fallback_allowed`
- `created_at`

#### `IntentAnnouncement`

Required fields:

- `intent_id`
- `session_id`
- `task_id`
- `resource_scope`
- `intent_kind`
- `created_at`
- `status`

#### `ResourceClaim`

Required fields:

- `claim_id`
- `session_id`
- `task_id`
- `resource_scope`
- `claim_mode`
- `created_at`
- `updated_at`
- `status`

#### `ConflictRecord`

Required fields:

- `conflict_id`
- `participant_refs`
- `related_resource_scope`
- `severity`
- `status`
- `coordination_channel_id`
- `created_at`
- `updated_at`

#### `CoordinationChannel`

Required fields:

- `coordination_channel_id`
- `participant_refs`
- `related_conflict_ids`
- `state`
- `created_at`
- `updated_at`

#### `MemoryRecordClass`

Required values:

- `exact`
- `derived`
- `provisional`
- `user_edited`

#### `SalienceScore`

Required fields:

- `subject_ref`
- `score`
- `factors`
- `computed_at`

#### `SupersessionLink`

Required fields:

- `supersession_id`
- `older_record_ref`
- `newer_record_ref`
- `reason`
- `created_at`

#### `CapabilityTrustState`

Required values:

- `staged`
- `enabled`
- `disabled`
- `revoked`

### State Machines

#### Daemon Lifecycle

Allowed states:

- `starting`
- `running`
- `stopping`
- `stopped`
- `degraded`

Allowed transitions:

- `starting -> running`
- `starting -> degraded`
- `running -> stopping`
- `running -> degraded`
- `degraded -> running`
- `stopping -> stopped`

#### `TaskState`

Allowed transitions:

- `submitted -> queued`
- `submitted -> failed`
- `queued -> running`
- `queued -> cancelled`
- `running -> awaiting_approval`
- `running -> blocked`
- `running -> succeeded`
- `running -> failed`
- `running -> cancelled`
- `awaiting_approval -> running`
- `awaiting_approval -> blocked`
- `awaiting_approval -> failed`
- `blocked -> failed`
- `blocked -> cancelled`

#### `StepState`

Allowed transitions:

- `proposed -> ready`
- `proposed -> blocked`
- `ready -> executing`
- `ready -> awaiting_approval`
- `executing -> completed`
- `executing -> failed`
- `awaiting_approval -> ready`
- `awaiting_approval -> blocked`
- `blocked -> failed`

#### `ApprovalState`

Allowed transitions:

- `pending -> approved`
- `pending -> denied`
- `pending -> expired`

#### `CapabilityTrustState`

Allowed transitions:

- `staged -> enabled`
- `staged -> disabled`
- `enabled -> disabled`
- `enabled -> revoked`
- `disabled -> enabled`
- `disabled -> revoked`

## 5. Event and Trace Spec

The trace system is the authoritative causal record for task execution. Every runtime-significant transition must result in a trace event.

### Mandatory Trace Emission Points

- session registered
- task submitted
- task admitted
- task queued
- step proposed
- route decision created
- policy decision created
- approval requested
- approval resolved
- capability execution started
- capability execution finished
- verification completed
- artifact emitted
- task state changed
- task completed or failed
- daemon restart recovery linked to task state
- intent announced
- resource claim recorded
- conflict detected or updated
- coordination state updated
- observability snapshot or health artifact emitted

### Ordering Rules

- Events are append-only within a task trace.
- `event_sequence` must be monotonic within a trace.
- Cross-task global ordering is not required for MVP.
- Recovery events must point back to the most recent durable task event before interruption.

### Correlation Rules

Every `TraceEvent` must include:

- `task_id`
- `step_id` where applicable
- `correlation_id` for grouping related work such as route, policy, capability, and verification for a single step

### Operator Retrieval

The operator must be able to:

- fetch the full trace for a task
- inspect ordered event summaries
- locate artifacts produced by specific events
- identify the last durable event before restart or failure

## 6. Artifact and Binding Spec

Artifacts and bindings are distinct.

- An `Artifact` is a durable, inspectable output record.
- A `Binding` is a runtime-owned handle to a value or reference used in execution.

### Binding Rules

- A binding may point to a raw executor result, external handle, path, or structured value.
- Bindings must not require the raw value to be included in model-visible text.
- Bindings must have one of the three visibility classes: `model_visible`, `engine_only`, or `approval_gated`.

### Artifact Rules

- Every artifact must include provenance to the producing step and trace event.
- Every final task result must be represented as an artifact.
- Derived summaries are optional in MVP, but if present they must point to exact source artifacts or trace events.
- Verification, observability, and cleanup-oriented artifacts are allowed in MVP and must follow the same provenance rules.

### MVP Artifact Kinds

- `route_decision`
- `capability_result`
- `verification_result`
- `failure_record`
- `final_result`
- `coordination_record`
- `observability_snapshot`
- `cleanup_candidate`

## 7. Policy Decision and Approval Spec

Policy is the explicit authority check between proposed work and execution.

### Minimum Policy Inputs

- task autonomy mode
- requested capability class
- route constraints
- current binding visibility
- side-effect class
- approval thresholds
- current coordination context where present

### Decision Outputs

- `allow`
- `deny`
- `require_approval`

### Approval Rules

- Approval is durable and restart-safe.
- Approval must be tied to one `Task` and one `Step`.
- Approval expiry must be explicit.
- Expired approvals cannot be reused.
- Denied approvals do not silently downgrade into allow.

### MVP Approval Behavior

- CLI lists pending approvals.
- CLI resolves approvals as approve or deny.
- Approved steps return to `ready` before execution.
- Denied steps transition the task to `blocked` or `failed` based on policy.

## 8. Capability Manifest Spec

Every capability available to the MVP must be declared by manifest.

### Required Manifest Fields

- `name`
- `version`
- `description`
- `executor_type`
- `input_schema`
- `output_contract`
- `side_effect_class`
- `scope`
- `timeout_ms`
- `cancellable`
- `trust_state`

### MVP Executor Types

MVP supports only:

- `local_builtin`

Deferred:

- `wasm_plugin`
- `mcp_remote`

### MVP Capability Set

The MVP must define exactly two capabilities:

1. `memory.read_context`
   - class: read-only
   - purpose: retrieve a small note or context item for a task

2. `artifact.write_note_draft`
   - class: restricted / side-effect-classified
   - purpose: produce a note-like artifact or write-intent output that requires policy evaluation

### Manifest Validation Rule

- A capability without a valid manifest is not executable.
- Invalid manifest state must be traceable as failure before execution starts.

## 9. Daemon Protocol Spec

The daemon protocol is the only external control contract for MVP.

### Minimum Operations

- `register-session`
- `submit-task`
- `get-task`
- `list-tasks`
- `control-task`
- `get-trace`
- `get-artifacts`
- `list-pending-approvals`
- `resolve-approval`

### Transport Posture

For MVP, the protocol may be implemented over a simple local RPC or CLI-to-daemon command interface. Transport sophistication is not required, but operation shapes must be stable.

### Idempotency

- `submit-task` must support an optional idempotency key.
- `resolve-approval` must be idempotent by `approval_id`.
- Replayed reads must be side-effect free.

### Response Requirements

All mutation operations must return:

- request acceptance or rejection
- stable identifiers
- current state summary
- reason on rejection

### Coordination Visibility

MVP does not require dedicated coordination commands. Instead:

- task, trace, and artifact reads must be able to expose linked intent, claim, conflict, or coordination references when they exist
- operators must be able to discover overlap or conflict state through standard retrieval paths
- protocol shapes must reserve stable fields for coordination summaries rather than force a later retrofit

## 10. Daemon Control Plane Spec

The daemon control plane owns process lifetime, session and task admission, task queueing, worker assignment, approval durability, minimal coordination record handling, and recovery.

### Responsibilities

- start and stop runtime services
- admit or reject sessions
- admit, queue, and supervise tasks
- attach runtime defaults
- persist top-level lifecycle state
- preserve approval state
- record intent, claim, conflict, and coordination metadata when overlap is detected or declared
- expose health and task listings

### MVP Queue Model

- single local queue
- one worker lane is sufficient
- tasks are processed in FIFO order unless cancelled

### Recovery Rules

- Restart restores task state, session state, approvals, and trace position from durable store.
- Lost worker does not imply completed task.
- In-flight step after restart must re-enter a recoverable explicit state, never implicit success.

### Conflict / Coordination Posture

Conflict resolution remains intentionally minimal in MVP, but coordination is not placeholder-only.

- the daemon may persist `IntentAnnouncement`, `ResourceClaim`, `ConflictRecord`, and `CoordinationChannel` records
- overlap detection may be simple rule-based or explicit-declaration-based in MVP
- the runtime must preserve and expose detected overlap, even when it does not arbitrate it automatically
- richer arbitration, locking, or negotiation flows are deferred

## 11. Agent Kernel Spec

The kernel owns task execution semantics.

### Kernel Loop

1. load task state
2. gather relevant runtime context
3. propose one step
4. request route and policy decisions
5. hand execution to the reasoning engine
6. verify outcome
7. update task state and emit trace
8. stop or continue

### MVP Kernel Rules

- only one step may execute at a time
- retries are bounded to one retry for MVP
- task success requires explicit verification result
- stop conditions must be explicit

### Terminal Outcomes

- `succeeded`
- `failed`
- `blocked`
- `cancelled`

## 12. Reasoning Engine Spec

The reasoning engine owns execution-time state for the current step, binding resolution, capability invocation, and output normalization.

### Responsibilities

- resolve step inputs from bindings
- check preconditions
- invoke one capability
- normalize outputs into bindings and artifacts
- classify outcome
- write execution-related trace segments

### MVP Outcome Classes

- `completed`
- `retryable_failure`
- `terminal_failure`
- `awaiting_approval`

### Postcondition Rule

Every successful capability execution must be followed by explicit verification before the task can be considered successful.

## 13. Capability Plane Spec

The capability plane is the boundary where manifest-backed actions are executed.

### MVP Capabilities

#### `memory.read_context`

- side-effect class: `read`
- policy expectation: normally allowed
- output: one `capability_result` artifact and optional binding

#### `artifact.write_note_draft`

- side-effect class: `restricted_write`
- policy expectation: must be evaluated before execution
- output: one `capability_result` artifact or one blocked outcome

### Runtime Rules

- manifest lookup is mandatory
- input validation occurs before execution
- timeout behavior is explicit
- outputs are normalized before returning to the kernel
- trust state must be `enabled` to execute

## 14. Memory System Spec

The MVP memory system must already embody the architecture's distinction between exact records, derived records, and bounded active state.

### Required MVP Stores

- session/task state store
- policy decision store
- approval store
- exact trace store
- artifact store
- binding reference store
- coordination record store

### Exact vs Derived Memory

Exact records:

- task state
- policy decisions
- approvals
- trace events
- capability output artifacts
- verification artifacts
- coordination records
- observability snapshots

Derived records:

- optional summaries
- optional note-style rollups
- optional cleanup candidates or drift summaries

### MVP Memory Rules

- exact records persist before derived records
- derived records may be absent, but their slot and provenance model must be defined
- bounded active state is represented by the current task state plus active bindings, not by replaying entire transcripts
- memory entries must be classifiable as `exact`, `derived`, `provisional`, or `user_edited`
- salience metadata and supersession links must have a stable schema even if MVP scoring and automation are simple
- salience-driven consolidation may be simple or mostly manual in MVP, but it must not be left structurally undefined
- explicit forgetting and supersession may remain minimal mechanisms, but the model must preserve provenance so they can be elaborated safely
- prior conflict and coordination history must be retrievable without reconstructing it from transcripts

## 15. Model Routing Spec

The MVP router exists even if routing logic is simple.

### MVP Route Model

- one local or mocked model profile
- one route decision per model-assisted step
- no hidden direct model call path

### Route Decision Inputs

- task class
- policy routing constraints
- model profile availability
- privacy posture

### Route Outputs

- selected `ModelProfile`
- `RouteDecision`
- `route_decision` artifact

### MVP Fallback

- if no route is allowed, the step is blocked or failed explicitly
- no silent downgrade to a hidden model path

## 16. Personality and Policy Spec

The MVP keeps personality and policy separate.

### Minimum `PersonalityProfile`

- `version`
- `voice_profile`
- `initiative_level`
- `risk_posture`

### Minimum `PolicyProfile`

- `version`
- `autonomy_mode_defaults`
- `approval_thresholds`
- `allowed_capability_classes`
- `routing_constraints`

### Boundary Rule

Personality may shape presentation but may never override a policy decision.

## 17. Autonomy and Approvals Spec

The MVP defines two active autonomy modes:

- `observe`
- `supervised`

Deferred:

- `autonomous`

### Mode Rules

#### `observe`

- read-oriented steps only
- restricted steps are denied or approval-gated

#### `supervised`

- read-oriented steps may proceed automatically
- restricted steps require policy evaluation and may require approval

### Approval Queue Rules

- pending approvals are durable
- CLI can inspect and resolve them
- approval resolution is idempotent
- denied or expired approvals do not leave the step ambiguous

## 18. Operator Surfaces Spec

CLI is the only primary MVP operator surface.

### Required CLI Commands

- `sharo daemon start`
- `sharo session open`
- `sharo task submit`
- `sharo task get`
- `sharo task list`
- `sharo trace get`
- `sharo artifacts list`
- `sharo approval list`
- `sharo approval resolve`
- `sharo task cancel`

### Operator Visibility Requirements

- current task state
- current step summary
- blocking reason
- route summary
- policy outcome summary
- coordination summary when present
- artifact references
- trace retrieval

### MVP Harness Hooks

The MVP surface must carry minimal harness hooks from the start:

- verification results must be retrievable as first-class artifacts
- observability snapshots or health records may be attached to traces and artifacts without changing protocol shape later
- cleanup candidates or drift markers may be emitted as artifacts even before recurring maintenance loops exist

TUI and Knot-facing UI are explicitly deferred, but this MVP surface must not require any private backdoor outside the daemon protocol.

## 19. Security Model Spec

The MVP security model is intentionally narrow but real.

### Required Security Posture

- raw sensitive values are not required in model-visible text
- capability scope is checked before execution
- restricted actions fail closed when classification is incomplete
- denied and blocked attempts are trace-visible

### MVP Enforcement Points

- manifest validation
- policy evaluation before restricted execution
- approval gate for restricted steps
- binding visibility handling

### Deferred Security Richness

- deep taint tracking
- advanced executor isolation
- secret vault sophistication
- cross-session access policies beyond basic scaffolding

## 20. MVP End-to-End Scenario Spec

Three scenarios are mandatory.

### Scenario A: Successful Read-Oriented Task

1. Operator starts daemon.
2. CLI opens a session.
3. CLI submits task: retrieve a small context item and report result.
4. Daemon admits task and assigns defaults.
5. Kernel proposes one read-oriented step.
6. Router records route decision.
7. Policy returns `allow`.
8. Reasoning engine invokes `memory.read_context`.
9. Capability returns normalized output.
10. Memory system persists trace and result artifact.
11. Verification artifact confirms postcondition.
12. Task transitions to `succeeded`.
13. CLI can inspect trace and artifacts.

### Scenario B: Restricted Task Blocked or Approval-Gated

1. Operator starts daemon and opens a session.
2. CLI submits task that requests note-draft write behavior.
3. Daemon admits task.
4. Kernel proposes one restricted step.
5. Router records route decision if model-assisted.
6. Policy returns `require_approval` or `deny`.
7. No restricted capability execution occurs before approval.
8. Task transitions to `awaiting_approval` or `blocked`.
9. CLI can inspect why the step did not proceed.
10. If approved, the step returns to `ready` and then executes.
11. If denied or expired, task remains explicitly blocked or failed.

### Scenario C: Overlap Recorded Without Built-In Arbitration

1. Operator opens two sessions through the daemon.
2. Each session submits a task that declares or triggers overlapping resource scope.
3. Daemon persists intent and claim records.
4. Daemon emits a `ConflictRecord` and links or creates a `CoordinationChannel`.
5. Neither task is silently rewritten or auto-resolved by hidden policy.
6. Standard task, trace, or artifact retrieval exposes the overlap state.
7. Restart preserves the overlap records and any visible uncertainty about incomplete coordination state.

## 21. Verification Matrix

| Invariant / Requirement | Subsystem | Scenario | Verification Type | Expected Evidence |
|---|---|---|---|---|
| Task has one durable state at a time | Core types / daemon | A, B | unit + integration | valid state transition log |
| Step ends explicitly | Kernel / reasoning | A, B | unit + integration | final `StepState` plus trace event |
| Policy checked before restricted execution | Policy / capability | B | integration | policy decision event before execution |
| Denied step does not execute | Policy / capability | B | integration | no execution event after deny |
| Route decision always recorded | Router | A, B | integration | `route_decision` artifact |
| Exact trace persists before derived memory | Memory | A | integration | trace events exist before summary artifacts |
| `PolicyDecision` and `Approval` persist as exact records | Memory / policy | A, B | integration | durable records linked to step and trace |
| Artifact provenance is queryable | Memory / trace | A | integration | artifact links to trace event |
| Coordination records persist when overlap is detected | Daemon / memory | C | integration | intent, claim, and conflict records linked to tasks |
| Overlap is visible without dedicated coordination commands | Protocol / operator surface | C | integration + operator check | task or trace retrieval shows coordination summary |
| Recovery preserves visible uncertainty for incomplete overlap state | Daemon / memory | C | recovery | restarted task still shows conflict or uncertain coordination state |
| Approval is restart-safe | Approvals / daemon | B | recovery | pending approval survives restart |
| Restart restores durable task state | Daemon / memory | A or B | recovery | same task id and state after restart |
| CLI can inspect blocking reason | Operator surface | B | operator check | command output shows explicit reason |
| Capability manifest required | Capability plane | A, B | unit + integration | invalid manifest blocks execution |
| Binding can remain opaque | Reasoning / memory | A | unit + integration | binding handle present without raw value leakage |
| Verification and observability artifacts are first-class | Harness / memory | A | integration | verification or health artifacts retrievable with provenance |

## 22. Enrichment Roadmap

Enrichment is risk-first and must be justified either by research or by observed MVP limitations.

### Milestone 1: Stabilize MVP Semantics

Focus:

- harden state transitions
- improve restart recovery
- strengthen trace integrity
- tighten manifest validation
- reduce ambiguity in failure handling
- add machine-checkable architecture rules for critical invariants
- harden the minimal observability and verification hooks already present in MVP

Justification:

- the MVP is only useful if its basic control invariants are reliable

New verification:

- stronger recovery scenarios
- transition legality checks
- trace continuity checks
- schema and contract checks for coordination, policy, and memory exact-record durability

### Milestone 2: Strengthen Memory Control

Focus:

- salience scoring
- consolidation jobs
- supersession and invalidation
- bounded active context packaging
- improved retrieval ranking

Research grounding:

- `Titans`
- `MIRAS`
- `Nested Learning / Hope`
- `Trainable Neural Memory`
- `Trellis`
- `MemGPT`
- `Reflexion`

New verification:

- exact-vs-derived provenance checks
- stale-memory invalidation tests
- bounded active context packaging checks

### Milestone 3: Harden Policy and Approvals

Focus:

- richer approval scopes
- tighter capability promotion gates
- stronger routing and execution gating
- external-content safety handling

Justification:

- once MVP control is stable, the next risk is unsafe broadening of authority

New verification:

- approval scope regression checks
- denied-path and expiry-path coverage
- capability trust-state transition checks

### Milestone 4: Broaden Operator Experience

Focus:

- TUI
- Knot bridge
- richer artifact views
- better trace and approval UX
- observability-facing capabilities beyond the MVP retrieval hooks
- recurring cleanup and doc-gardening workflows

Justification:

- stronger operator visibility increases usable oversight without changing core authority

New verification:

- client resync scenarios
- visibility consistency checks
- artifact and trace inspection UX coverage
- recurring cleanup artifact and drift-marker coverage

### Milestone 5: Controlled Power Expansion

Focus:

- broader capability catalog
- stronger MCP integration
- scheduler maturity
- bounded delegated work

Justification:

- power expansion is only acceptable after stability, memory control, and gating are credible

New verification:

- capability regression suites
- policy containment checks
- scheduling and delegation recovery checks

### Rule For Every Enrichment

Every enrichment must answer:

- what user-visible limitation or invariant gap it addresses
- what research or operational evidence justifies it
- what new failure modes it introduces
- what new verification is required

## 23. Assumptions and Defaults

- canonical local path: `docs/specs/mvp.md`
- vault path is non-canonical and may diverge unless an explicit sync is requested
- explicit repo-vault sync should follow `docs/specs/vault-sync-protocol.md`
- CLI is the first operator surface
- all architecture parts are present in MVP
- the runtime is a thin vertical slice, not a broad assistant
- enrichment is risk-first
- enrichment must be research-grounded or operationally justified
