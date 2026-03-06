use sharo_core::client::{RuntimeClient, StubClient};
use sharo_core::protocol::{SubmitTaskRequest, TaskStatusRequest};

#[test]
fn stub_submit_is_deterministic_for_goal_and_session() {
    let client = StubClient;

    let request = SubmitTaskRequest {
        session_id: Some("session-a".to_string()),
        goal: "read docs".to_string(),
    };

    let first = client.submit(&request);
    let second = client.submit(&request);

    assert_eq!(first.task_id, second.task_id);
    assert_eq!(first.state, second.state);
}

#[test]
fn stub_status_is_deterministic_for_task_id() {
    let client = StubClient;
    let request = TaskStatusRequest {
        task_id: "task-0001".to_string(),
    };

    let first = client.status(&request);
    let second = client.status(&request);

    assert_eq!(first.task_id, second.task_id);
    assert_eq!(first.state, second.state);
    assert_eq!(first.summary, second.summary);
}
