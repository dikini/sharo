use sharo_core::runtime_types::{
    ArtifactKind, ArtifactRecord, StepRecord, StepState, TaskRecord, TaskStateV2, TraceEvent,
    TraceRecord,
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
