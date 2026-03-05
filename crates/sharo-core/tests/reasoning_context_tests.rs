use sharo_core::reasoning::ReasoningInput;
use sharo_core::reasoning_context::{
    AdjustmentPlan, AdjustmentStep, AlwaysFitPolicyFitter, ComposePrompt, Composer,
    ContextState, FitDecision, NoOpComposer, ReasoningContextError, TurnScope, run_fit_loop,
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
fn id_reasoning_engine_compatibility_with_context_defaults() {
    let input = ReasoningInput {
        trace_id: "trace-task-1".to_string(),
        task_id: "task-1".to_string(),
        goal: "read one context item".to_string(),
    };

    let state = ContextState::from_reasoning_input_defaults(&input);
    let prompt = NoOpComposer.compose(&state);
    assert_eq!(prompt.prompt_text, input.goal);
}
