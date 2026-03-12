# Workflow CI Runtime Optimization

> Template policy (brief):
> - Keep `Template-Profile: tdd-strict-v1`.
> - Define tests before implementation work.
> - Every task must include Preconditions, Invariants, and Postconditions.
> - Use Unit, Invariant, and Integration checks.
> - Use Property-based tests only when a generative framework is actually used.
> - Run `scripts/doc-lint.sh` before commit.

Updated: 2026-03-12
Status: active
Owner: platform
Template-Profile: tdd-strict-v1

Task-Registry-Refs: TASK-WORKFLOW-CI-RUNTIME-SPEC-001, TASK-WORKFLOW-CI-RUNTIME-PLAN-001

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level workflow and verification policies.
3. This spec's CI runtime-optimization contracts.
4. Explicit updates recorded in this document.

## Output Contract

- Reduce per-push `policy-checks` runtime without weakening merge-blocking confidence.
- Stop compiling or installing CI tools when the run will skip their checks anyway.
- Remove duplicated Rust verification surfaces inside `policy-checks`.
- Move nightly-only fuzz/toolchain work out of the per-push workflow.

## Evidence / Verification Contract

- Every optimization claim must cite measured timing or job-log evidence from a real workflow run.
- If a per-push check is moved or removed, the replacement workflow or remaining coverage surface must be stated explicitly.
- Cache recommendations must distinguish install-time waste from Rust workspace compile/test waste.

## Model Compatibility Notes

- This slice changes CI/workflow structure and shell entrypoints, not Rust runtime behavior.
- Rust-specific recommendations should preserve deterministic test semantics and avoid hiding failures behind overly broad skip logic.

## Purpose

Define the next workflow-hardening pass after local-first linting and range-sensitive skips: remove unnecessary tool compilation, reduce redundant Rust test execution, improve cache effectiveness, and move nightly-dependent fuzz work into a dedicated nightly workflow.

## Scope

### In Scope

- Gating CI tool installation before install steps for dependency-security and fuzz/nightly surfaces.
- Moving fuzz and nightly-toolchain setup out of per-push `policy-checks` into a separate nightly workflow.
- Removing duplicated property/loom coverage from `policy-checks` when already covered by workspace tests, or making one surface the single source of truth.
- Adding a lighter CI-specific verification entrypoint when `just verify` duplicates later workflow steps.
- Recording cache-effectiveness and project-structure constraints that materially affect compile reuse.

### Out of Scope

- Removing dependency-security, property tests, loom checks, or fuzz coverage from the repository entirely.
- Large Rust module refactors done only for aesthetics.
- Replacing `sccache` with a different compiler cache system.

## Core Terms

- `Pre-Install Gating`: deciding whether a tool lane is needed before running `cargo install` or `rustup toolchain install`.
- `Duplicate Rust Coverage`: the same logical test or target being executed once inside workspace tests and again in dedicated steps.
- `Nightly-Only Verification`: checks that require `cargo-fuzz` or a nightly toolchain and do not need to block every push.
- `CI Smoke Entrypoint`: a lightweight canonical verification command for CI that avoids duplicating later dedicated steps.

## Interfaces / Contracts

- `.github/workflows/policy-checks.yml` remains the merge-blocking per-push workflow, but should not install tools for lanes that will skip.
- A new nightly workflow owns nightly toolchain installation and fuzz execution.
- `scripts/check-dependencies-security.sh --range <git-range>` remains the canonical dependency-security entrypoint.
- `scripts/check-fuzz.sh` remains the canonical fuzz entrypoint, but per-push and nightly callers may use different modes.
- `scripts/check-tests.sh` and `scripts/check-rust-tests.sh` remain the canonical Rust test entrypoints.
- If introduced, `just verify-ci` must be narrower than local `just verify` and documented as CI-only smoke coverage.

## Invariants

- Merge-blocking CI must still catch real Rust, docs, task-sync, shell, and policy regressions for relevant changes.
- No tool should be installed in CI if the range proves its lane will skip.
- The same Rust test target should not be intentionally executed twice in one `policy-checks` run unless the duplication is justified and documented.
- Nightly/fuzz checks remain automated, but in a separate workflow with explicit schedule/manual triggers.
- Cache guidance must remain compatible with `sccache` and Rust 2024 workspace behavior.

## Task Contracts

### Task 1: Define pre-install gating for heavy CI tool lanes

**Preconditions**

- Current `policy-checks` timing evidence is available from a successful post-rollout run.
- Dependency-security and fuzz/nightly lanes already support skip semantics after install.

**Invariants**

- Cargo-input changes still trigger dependency-security installation and execution.
- Fuzz-impacting changes still trigger the required fuzz toolchain in whichever workflow owns them.

**Postconditions**

- The spec defines gating before `cargo install` and `rustup toolchain install`, not only before execution.

**Tests (must exist before implementation)**

Unit:
- `policy_checks_installs_dependency_tools_only_for_cargo_ranges`
- `policy_checks_installs_fuzz_toolchain_only_when_fuzz_lane_runs`

Invariant:
- `range_classification_happens_before_heavy_install_steps`

Integration:
- `docs_only_policy_checks_run_skips_dependency_and_fuzz_installs`

Property-based (optional):
- not applicable

### Task 2: Define the per-push vs nightly verification split

**Preconditions**

- The team accepts moving fuzz/nightly work into a nightly workflow.

**Invariants**

- Per-push `policy-checks` remains merge-blocking and fast.
- Nightly workflow retains automated coverage for fuzz/nightly-only surfaces.

**Postconditions**

- The spec defines which checks stay in `policy-checks` and which move to nightly.

**Tests (must exist before implementation)**

Unit:
- `policy_checks_excludes_nightly_fuzz_steps`
- `nightly_workflow_runs_fuzz_and_nightly_toolchain_steps`

Invariant:
- `every_removed_policy_check_step_has_named_replacement_workflow_or_remaining_cover`

Integration:
- `nightly_workflow_replays_fuzz_smoke_or_full_targets_without_policy_checks_dependency`

Property-based (optional):
- not applicable

### Task 3: Define duplicate-Rust-coverage removal and cache-aware execution

**Preconditions**

- Job logs identify repeated property/loom execution and repeated crate rebuilds across separate steps.

**Invariants**

- Required Rust verification remains covered exactly once per workflow path unless duplication is justified.
- CI cache settings remain explicit and deterministic.

**Postconditions**

- The spec defines one source of truth for property/loom coverage per workflow and records cache-oriented recommendations such as `CARGO_INCREMENTAL=0` in CI.

**Tests (must exist before implementation)**

Unit:
- `policy_checks_does_not_run_property_target_twice`
- `policy_checks_does_not_run_loom_target_twice`
- `ci_sets_cargo_incremental_zero_when_using_sccache`

Invariant:
- `workspace_test_entrypoint_and_dedicated_steps_have_no_unjustified_overlap`

Integration:
- `policy_checks_runtime_drops_after_duplicate_rust_coverage_removal`

Property-based (optional):
- not applicable

## Scenarios

- S1: A docs-only push should skip dependency-security install, skip fuzz/nightly install, skip shell tests when unrelated, and still run merge-blocking docs/task/policy checks.
- S2: A push that changes `Cargo.lock` should install and run dependency-security, but should not pay nightly/fuzz costs unless fuzz inputs changed and remain in-scope for that workflow.
- S3: A regular Rust feature push should run workspace tests once and should not rerun the same property/loom targets in later dedicated steps.
- S4: The nightly workflow should install the nightly toolchain and `cargo-fuzz`, then run the designated fuzz/nightly verification without relying on `policy-checks`.

## Timing Evidence

- Post-rollout successful run `23010870993` (`4063caa`): total job time about `7m52s`.
- Dominant step costs in that run:
  - `Install dependency security tools`: `306s`
  - `Run Rust workspace tests`: `50s`
  - `Run canonical verification entrypoint`: `49s`
  - `Install cargo-fuzz`: `8s`
  - `Install nightly toolchain for fuzzing`: `13s`
  - `Run property test profile`: `11s`
  - `Run loom model checks`: `3s`
- Job-log evidence from the same run:
  - dependency-security execution skipped, but install still consumed the full `306s`
  - fuzz execution skipped, but `cargo-fuzz` and nightly installation still consumed `21s`
  - `prop_protocol_roundtrip_preserves_task_summary_fields` ran inside workspace tests and then again in a dedicated step
  - `loom_submit_shutdown` ran inside workspace tests and then again in a dedicated step
- `sccache` evidence from the same run:
  - overall hit rate `89.91%`
  - Rust hit rate `83.98%`
  - Rust misses `137`
  - non-cacheable reasons dominated by `crate-type` (`159`) and `incremental` (`52`)

## Verification

- `scripts/doc-lint.sh --changed --strict-new`
- `scripts/check-tasks-registry.sh`
- `scripts/check-tasks-sync.sh --changed`
- `scripts/check-fast-feedback.sh`

## Risks and Failure Modes

- Moving too much out of `policy-checks` can delay detection of real merge-blocking regressions if the split is not explicit.
- Over-fitting path gates can hide required installs for edge-case file movements or indirect dependency changes.
- Reducing duplicate Rust coverage without checking target overlap carefully can accidentally drop unique verification.
- Nightly workflow drift can let fuzz coverage rot if it is not visible and monitored.

## Open Questions

- Should property/loom stay in `policy-checks` as dedicated steps and be excluded from workspace tests, or should the dedicated steps be deleted because workspace tests already cover them?
- Should `policy-checks` stay single-job after these reductions, or is a later split into parallel jobs still worth the extra complexity?

## References

- [workflow-ci-latency-hardening.md](workflow-ci-latency-hardening.md)
- [policy-checks run 23010870993](https://github.com/dikini/sharo/actions/runs/23010870993)
