use sharo_core::runtime_types::{
    ArtifactKind, ArtifactRecord, BindingRecord, BindingVisibility, StepRecord, StepState,
    TaskRecord, TaskStateV2, TraceEvent, TraceRecord,
};

#[test]
fn task_state_supports_scenario_a_transitions() {
    assert!(TaskStateV2::Submitted.can_transition_to(TaskStateV2::Queued));
    assert!(TaskStateV2::Queued.can_transition_to(TaskStateV2::Running));
    assert!(TaskStateV2::Running.can_transition_to(TaskStateV2::Succeeded));
    assert!(!TaskStateV2::Succeeded.can_transition_to(TaskStateV2::Running));
}

#[test]
fn trace_event_sequence_is_monotonic() {
    let mut trace = TraceRecord::new("trace-1", "task-1", "session-1");

    trace.push_event(TraceEvent::new(1, "task_submitted", "submitted"));
    trace.push_event(TraceEvent::new(2, "route_decision", "local_mock"));
    trace.push_event(TraceEvent::new(3, "verification_completed", "ok"));

    assert!(trace.is_monotonic());

    trace.push_event(TraceEvent::new(2, "bad", "out_of_order"));
    assert!(!trace.is_monotonic());
}

#[test]
fn scenario_a_record_roundtrip_json() {
    let task = TaskRecord {
        task_id: "task-1".to_string(),
        session_id: "session-1".to_string(),
        goal: "read context".to_string(),
        task_state: TaskStateV2::Succeeded,
        current_step_id: Some("step-1".to_string()),
        result_artifact_id: Some("artifact-final".to_string()),
    };

    let step = StepRecord {
        step_id: "step-1".to_string(),
        task_id: task.task_id.clone(),
        step_state: StepState::Completed,
        summary: "read one context item".to_string(),
        requested_capability: "memory.read_context".to_string(),
    };

    let artifact = ArtifactRecord {
        artifact_id: "artifact-verification".to_string(),
        task_id: task.task_id.clone(),
        artifact_kind: ArtifactKind::VerificationResult,
        summary: "postconditions satisfied".to_string(),
        content: "verified".to_string(),
        produced_by_step_id: step.step_id.clone(),
        produced_by_trace_event_sequence: 3,
    };

    let payload = serde_json::to_string(&(task, step, artifact)).expect("serialize");
    let decoded: (TaskRecord, StepRecord, ArtifactRecord) =
        serde_json::from_str(&payload).expect("deserialize");

    assert_eq!(decoded.0.task_id, "task-1");
    assert_eq!(decoded.1.step_id, "step-1");
    assert_eq!(decoded.2.artifact_kind, ArtifactKind::VerificationResult);
}

#[test]
fn step_terminal_state_is_explicit() {
    let terminal = [StepState::Completed, StepState::Blocked, StepState::Failed];
    let non_terminal = [
        StepState::Proposed,
        StepState::Ready,
        StepState::Executing,
        StepState::AwaitingApproval,
    ];

    for state in terminal {
        assert!(matches!(
            state,
            StepState::Completed | StepState::Blocked | StepState::Failed
        ));
    }
    for state in non_terminal {
        assert!(!matches!(
            state,
            StepState::Completed | StepState::Blocked | StepState::Failed
        ));
    }
}

#[test]
fn binding_visibility_redacts_non_model_values() {
    let engine_only = BindingRecord {
        binding_id: "binding-1".to_string(),
        task_id: "task-1".to_string(),
        step_id: "step-1".to_string(),
        visibility: BindingVisibility::EngineOnly,
        handle: "engine-handle-1".to_string(),
        raw_value_model_text: None,
        raw_value_redacted: true,
    };
    let approval_gated = BindingRecord {
        binding_id: "binding-2".to_string(),
        task_id: "task-1".to_string(),
        step_id: "step-1".to_string(),
        visibility: BindingVisibility::ApprovalGated,
        handle: "approval-handle-1".to_string(),
        raw_value_model_text: None,
        raw_value_redacted: true,
    };

    assert!(!engine_only.is_model_text_exposed());
    assert!(!approval_gated.is_model_text_exposed());
    assert!(engine_only.raw_value_redacted);
    assert!(approval_gated.raw_value_redacted);
}

#[test]
fn binding_model_visible_can_expose_model_text() {
    let model_visible = BindingRecord {
        binding_id: "binding-3".to_string(),
        task_id: "task-2".to_string(),
        step_id: "step-2".to_string(),
        visibility: BindingVisibility::ModelVisible,
        handle: "visible-handle".to_string(),
        raw_value_model_text: Some("safe-visible-value".to_string()),
        raw_value_redacted: false,
    };

    assert!(model_visible.is_model_text_exposed());
    assert_eq!(
        model_visible.raw_value_model_text.as_deref(),
        Some("safe-visible-value")
    );
}

#[test]
fn binding_handle_present_when_value_redacted() {
    let binding = BindingRecord {
        binding_id: "binding-4".to_string(),
        task_id: "task-3".to_string(),
        step_id: "step-3".to_string(),
        visibility: BindingVisibility::EngineOnly,
        handle: "engine-handle-opaque".to_string(),
        raw_value_model_text: None,
        raw_value_redacted: true,
    };

    assert!(!binding.handle.is_empty());
    assert!(binding.raw_value_model_text.is_none());
}
