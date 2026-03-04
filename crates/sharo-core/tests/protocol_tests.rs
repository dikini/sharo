use sharo_core::protocol::{
    SubmitTaskRequest, SubmitTaskResponse, TaskState, TaskStatusRequest, TaskStatusResponse,
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
