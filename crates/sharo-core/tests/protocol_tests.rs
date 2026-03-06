use proptest::prelude::*;
use sharo_core::protocol::{
    SubmitTaskRequest, SubmitTaskResponse, TaskState, TaskStatusRequest, TaskStatusResponse,
    TaskSummary,
};

#[test]
fn submit_request_response_roundtrip() {
    let request = SubmitTaskRequest {
        session_id: Some("session-a".to_string()),
        goal: "read docs".to_string(),
    };

    let response = SubmitTaskResponse {
        task_id: "task-0001".to_string(),
        state: TaskState::Submitted,
    };

    assert_eq!(request.session_id.as_deref(), Some("session-a"));
    assert_eq!(request.goal, "read docs");
    assert_eq!(response.task_id, "task-0001");
    assert_eq!(response.state, TaskState::Submitted);
}

#[test]
fn status_request_response_roundtrip() {
    let request = TaskStatusRequest {
        task_id: "task-0007".to_string(),
    };

    let response = TaskStatusResponse {
        task_id: request.task_id.clone(),
        state: TaskState::Running,
        summary: "in progress".to_string(),
    };

    assert_eq!(response.task_id, "task-0007");
    assert_eq!(response.state, TaskState::Running);
    assert_eq!(response.summary, "in progress");
}

#[test]
fn protocol_includes_optional_coordination_summary() {
    let task = TaskSummary {
        task_id: "task-42".to_string(),
        session_id: "session-1".to_string(),
        task_state: "awaiting_approval".to_string(),
        current_step_summary: "restricted write pending approval".to_string(),
        blocking_reason: Some("approval_required approval_id=approval-000001".to_string()),
        coordination_summary: Some(
            "conflict_id=conflict-000001 scope=notes related_task_id=task-41".to_string(),
        ),
        result_preview: None,
    };

    let payload = serde_json::to_string(&task).expect("serialize task summary");
    let roundtrip: TaskSummary = serde_json::from_str(&payload).expect("parse task summary");
    assert_eq!(roundtrip.coordination_summary, task.coordination_summary);
}

proptest! {
    #[test]
    fn prop_protocol_roundtrip_preserves_task_summary_fields(
        task_id in "[a-z0-9\\-]{1,24}",
        session_id in "[a-z0-9\\-]{1,24}",
        task_state in "[a-z_]{1,24}",
        summary in ".{0,64}",
    ) {
        let task = TaskSummary {
            task_id: task_id.clone(),
            session_id: session_id.clone(),
            task_state: task_state.clone(),
            current_step_summary: summary.clone(),
            blocking_reason: Some("reason".to_string()),
            coordination_summary: Some("coord".to_string()),
            result_preview: Some("preview".to_string()),
        };

        let payload = serde_json::to_string(&task).expect("serialize task summary");
        let roundtrip: TaskSummary = serde_json::from_str(&payload).expect("parse task summary");

        prop_assert_eq!(roundtrip.task_id, task_id);
        prop_assert_eq!(roundtrip.session_id, session_id);
        prop_assert_eq!(roundtrip.task_state, task_state);
        prop_assert_eq!(roundtrip.current_step_summary, summary);
        prop_assert_eq!(roundtrip.result_preview, Some("preview".to_string()));
    }
}
