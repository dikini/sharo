use sharo_core::protocol::{
    ControlTaskRequest, ControlTaskResponse, DaemonInfoRequest, DaemonRequest, DaemonResponse,
    SubmitTaskOpRequest, SubmitTaskOpResponse,
};

#[test]
fn protocol_contains_required_mvp_operations() {
    let _ = DaemonRequest::DaemonInfo(DaemonInfoRequest {});
    let _ = DaemonRequest::ControlTask(ControlTaskRequest {
        task_id: "task-1".to_string(),
        action: "cancel".to_string(),
    });
}

#[test]
fn mutation_response_contains_acceptance_and_reason_fields() {
    let submit = SubmitTaskOpResponse {
        task_id: "task-1".to_string(),
        task_state: "submitted".to_string(),
        accepted: true,
        reason: Some("accepted".to_string()),
        summary: "task admitted".to_string(),
    };
    assert!(submit.accepted);
    assert_eq!(submit.reason.as_deref(), Some("accepted"));

    let control = ControlTaskResponse {
        task_id: "task-1".to_string(),
        task_state: "cancelled".to_string(),
        accepted: true,
        reason: "cancelled_by_operator".to_string(),
        summary: "control_applied".to_string(),
    };
    assert!(control.accepted);
    assert_eq!(control.reason, "cancelled_by_operator");
}

#[test]
fn protocol_envelope_roundtrip_for_all_operations() {
    let submit = DaemonRequest::SubmitTask(SubmitTaskOpRequest {
        session_id: Some("session-1".to_string()),
        goal: "read docs".to_string(),
        idempotency_key: Some("idem-1".to_string()),
    });
    let submit_json = serde_json::to_string(&submit).expect("serialize submit");
    let _: DaemonRequest = serde_json::from_str(&submit_json).expect("deserialize submit");

    let control = DaemonRequest::ControlTask(ControlTaskRequest {
        task_id: "task-1".to_string(),
        action: "cancel".to_string(),
    });
    let control_json = serde_json::to_string(&control).expect("serialize control");
    let _: DaemonRequest = serde_json::from_str(&control_json).expect("deserialize control");

    let response = DaemonResponse::ControlTask(ControlTaskResponse {
        task_id: "task-1".to_string(),
        task_state: "cancelled".to_string(),
        accepted: true,
        reason: "cancelled_by_operator".to_string(),
        summary: "control_applied".to_string(),
    });
    let response_json = serde_json::to_string(&response).expect("serialize response");
    let _: DaemonResponse = serde_json::from_str(&response_json).expect("deserialize response");
}
