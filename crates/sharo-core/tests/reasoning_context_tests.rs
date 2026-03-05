use sharo_core::reasoning::ReasoningInput;
use sharo_core::reasoning_context::{
    AdjustmentPlan, AdjustmentStep, AlwaysFitPolicyFitter, ComposePrompt, Composer, ContextState,
    FitDecision, HeuristicPolicyFitter, NoOpComposer, PolicyConfig, PolicyFitter,
    ReasoningContextError,
    TurnScope, run_fit_loop,
};

#[test]
fn turn_scope_excludes_derived_fields() {
    let scope = TurnScope {
        session_id: "session-1".to_string(),
        task_id: "task-1".to_string(),
        turn_id: 1,
        goal: "read docs".to_string(),
    };

    assert_eq!(scope.session_id, "session-1");
    assert_eq!(scope.task_id, "task-1");
    assert_eq!(scope.turn_id, 1);
    assert_eq!(scope.goal, "read docs");
}

#[test]
fn fit_loop_stops_on_fitted_or_max_iters() {
    let mut state = ContextState::default_with_goal("goal".to_string());
    let composer = NoOpComposer;
    let fitter = AlwaysFitPolicyFitter;
    let mut applier = sharo_core::reasoning_context::DeterministicAdjustmentApplier;

    let result = run_fit_loop(&mut state, &composer, &fitter, &mut applier, 2)
        .expect("always-fit should converge immediately");
    assert_eq!(result.iterations, 1);
    assert_eq!(result.prompt.prompt_text, "goal");

    struct NeverFit;
    impl sharo_core::reasoning_context::PolicyFitter for NeverFit {
        fn fit(&self, _prompt: &ComposePrompt, _state: &ContextState) -> FitDecision {
            FitDecision::Adjust(AdjustmentPlan {
                plan_id: "adjust-1".to_string(),
                rationale: "force non-convergence".to_string(),
                steps: vec![AdjustmentStep::ClampPersonaVerbosity {
                    level: "low".to_string(),
                }],
            })
        }
    }

    let mut state = ContextState::default_with_goal("goal".to_string());
    let err = run_fit_loop(&mut state, &composer, &NeverFit, &mut applier, 1)
        .expect_err("max-iter guard should fail");
    assert!(matches!(
        err,
        ReasoningContextError::ContextPolicyFitFailed(_)
    ));
}

#[test]
fn fit_loop_state_hash_progress_is_monotonic_or_fails() {
    struct RepeatingPlanFitter;
    impl sharo_core::reasoning_context::PolicyFitter for RepeatingPlanFitter {
        fn fit(&self, _prompt: &ComposePrompt, _state: &ContextState) -> FitDecision {
            FitDecision::Adjust(AdjustmentPlan {
                plan_id: "noop".to_string(),
                rationale: "no-op".to_string(),
                steps: vec![],
            })
        }
    }

    let mut state = ContextState::default_with_goal("goal".to_string());
    let composer = NoOpComposer;
    let fitter = RepeatingPlanFitter;
    let mut applier = sharo_core::reasoning_context::DeterministicAdjustmentApplier;

    let err = run_fit_loop(&mut state, &composer, &fitter, &mut applier, 3)
        .expect_err("non-progress should fail");
    assert!(matches!(
        err,
        ReasoningContextError::NonProgressDetected(_)
    ));
}

#[test]
fn state_hash_uses_collision_safe_encoding_for_delimiter_rich_values() {
    let a = ContextState {
        system: "x|persona=y".to_string(),
        persona: "z".to_string(),
        memory: "m".to_string(),
        runtime: "r".to_string(),
        goal: "g".to_string(),
    };
    let b = ContextState {
        system: "x".to_string(),
        persona: "y|persona=z".to_string(),
        memory: "m".to_string(),
        runtime: "r".to_string(),
        goal: "g".to_string(),
    };

    assert_ne!(a.state_hash(), b.state_hash());
}

#[test]
fn state_hash_does_not_expose_raw_context_text() {
    let state = ContextState {
        system: "super-secret-system".to_string(),
        persona: "super-secret-persona".to_string(),
        memory: "super-secret-memory".to_string(),
        runtime: "super-secret-runtime".to_string(),
        goal: "super-secret-goal".to_string(),
    };

    let fingerprint = state.state_hash();
    assert!(!fingerprint.contains("super-secret"));
    assert_eq!(fingerprint.len(), 64);
}

#[test]
fn heuristic_policy_fitter_emits_adjustments_for_budget_and_runtime_redaction() {
    let fitter = HeuristicPolicyFitter::new(PolicyConfig {
        max_prompt_chars: 10,
        max_memory_lines: 1,
        forbidden_runtime_fields: vec!["token".to_string()],
    });

    let state = ContextState {
        system: String::new(),
        persona: "verbosity=high".to_string(),
        memory: "one\ntwo".to_string(),
        runtime: "token=abc".to_string(),
        goal: "goal".to_string(),
    };
    let prompt = ComposePrompt {
        prompt_text: "this is longer than ten".to_string(),
    };

    let decision = fitter.fit(&prompt, &state);
    match decision {
        FitDecision::Adjust(plan) => {
            assert!(plan.steps.iter().any(|s| matches!(
                s,
                AdjustmentStep::RedactRuntimeFields { .. }
            )));
            assert!(plan.steps.iter().any(|s| matches!(
                s,
                AdjustmentStep::DropMemoryByRank { .. }
            )));
            assert!(plan.steps.iter().any(|s| matches!(
                s,
                AdjustmentStep::CompressMemoryToTokens { .. }
            )));
            assert!(plan.steps.iter().any(|s| matches!(
                s,
                AdjustmentStep::ClampPersonaVerbosity { .. }
            )));
        }
        other => panic!("unexpected decision: {other:?}"),
    }
}

#[test]
fn id_reasoning_engine_compatibility_with_context_defaults() {
    let input = ReasoningInput {
        trace_id: "trace-task-1".to_string(),
        task_id: "task-1".to_string(),
        session_id: "session-1".to_string(),
        turn_id: 1,
        goal: "read one context item".to_string(),
        metadata: Default::default(),
    };

    let state = ContextState::from_reasoning_input_defaults(&input);
    let prompt = NoOpComposer.compose(&state);
    assert_eq!(prompt.prompt_text, input.goal);
}
