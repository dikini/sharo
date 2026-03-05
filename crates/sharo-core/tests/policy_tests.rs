use sharo_core::policy::{
    ActionClass, PolicyContext, PolicyDecisionKind, PolicyEngine,
};

#[test]
fn restricted_action_requires_policy_decision() {
    let engine = PolicyEngine::default();

    let allow = engine.evaluate(&PolicyContext {
        task_id: "task-1".to_string(),
        step_id: "step-1".to_string(),
        action_class: ActionClass::Read,
        autonomy_mode: "supervised".to_string(),
    });

    let approval = engine.evaluate(&PolicyContext {
        task_id: "task-1".to_string(),
        step_id: "step-2".to_string(),
        action_class: ActionClass::RestrictedWrite,
        autonomy_mode: "supervised".to_string(),
    });

    assert_eq!(allow.decision, PolicyDecisionKind::Allow);
    assert_eq!(approval.decision, PolicyDecisionKind::RequireApproval);
}

#[test]
fn policy_decision_is_deterministic_for_same_input() {
    let engine = PolicyEngine::default();

    let context = PolicyContext {
        task_id: "task-9".to_string(),
        step_id: "step-7".to_string(),
        action_class: ActionClass::RestrictedWrite,
        autonomy_mode: "observe".to_string(),
    };

    let first = engine.evaluate(&context);
    let second = engine.evaluate(&context);

    assert_eq!(first.decision, second.decision);
    assert_eq!(first.reason, second.reason);
}
