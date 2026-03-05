use sharo_core::protocol::{
    ArtifactSummary, DaemonRequest, DaemonResponse, GetArtifactsResponse, GetTraceResponse,
    ResolveApprovalRequest, ResolveApprovalResponse, SubmitTaskRequest, SubmitTaskResponse,
    TaskState, TaskStatusRequest, TaskStatusResponse, TraceEventSummary, TraceSummary,
};

#[test]
fn ipc_submit_envelope_roundtrip() {
    let request = DaemonRequest::Submit(SubmitTaskRequest {
        session_id: Some("session-a".to_string()),
        goal: "read docs".to_string(),
    });

    let json = serde_json::to_string(&request).expect("serialize submit request");
    let parsed: DaemonRequest = serde_json::from_str(&json).expect("deserialize submit request");

    match parsed {
        DaemonRequest::Submit(payload) => {
            assert_eq!(payload.session_id.as_deref(), Some("session-a"));
            assert_eq!(payload.goal, "read docs");
        }
        _ => panic!("expected submit request"),
    }
}

#[test]
fn ipc_status_envelope_roundtrip() {
    let response = DaemonResponse::Status(TaskStatusResponse {
        task_id: "task-0001".to_string(),
        state: TaskState::Succeeded,
        summary: "completed".to_string(),
    });

    let json = serde_json::to_string(&response).expect("serialize status response");
    let parsed: DaemonResponse = serde_json::from_str(&json).expect("deserialize status response");

    match parsed {
        DaemonResponse::Status(payload) => {
            assert_eq!(payload.task_id, "task-0001");
            assert_eq!(payload.state, TaskState::Succeeded);
            assert_eq!(payload.summary, "completed");
        }
        _ => panic!("expected status response"),
    }
}

#[test]
fn response_variant_matches_request_kind() {
    let submit_req = DaemonRequest::Submit(SubmitTaskRequest {
        session_id: None,
        goal: "g".to_string(),
    });
    let submit_resp = DaemonResponse::Submit(SubmitTaskResponse {
        task_id: "task-1".to_string(),
        state: TaskState::Submitted,
    });

    let status_req = DaemonRequest::Status(TaskStatusRequest {
        task_id: "task-1".to_string(),
    });
    let status_resp = DaemonResponse::Status(TaskStatusResponse {
        task_id: "task-1".to_string(),
        state: TaskState::Running,
        summary: "in progress".to_string(),
    });

    assert!(matches!(submit_req, DaemonRequest::Submit(_)));
    assert!(matches!(submit_resp, DaemonResponse::Submit(_)));
    assert!(matches!(status_req, DaemonRequest::Status(_)));
    assert!(matches!(status_resp, DaemonResponse::Status(_)));
}

#[test]
fn approval_envelope_roundtrip() {
    let resolve_req = DaemonRequest::ResolveApproval(ResolveApprovalRequest {
        approval_id: "approval-000001".to_string(),
        decision: "approve".to_string(),
    });
    let resolve_req_json = serde_json::to_string(&resolve_req).expect("serialize resolve request");
    let resolve_req_parsed: DaemonRequest =
        serde_json::from_str(&resolve_req_json).expect("deserialize resolve request");
    assert!(matches!(resolve_req_parsed, DaemonRequest::ResolveApproval(_)));

    let resolve_resp = DaemonResponse::ResolveApproval(ResolveApprovalResponse {
        approval_id: "approval-000001".to_string(),
        task_id: "task-000001".to_string(),
        state: "approved".to_string(),
    });
    let resolve_resp_json = serde_json::to_string(&resolve_resp).expect("serialize resolve response");
    let resolve_resp_parsed: DaemonResponse =
        serde_json::from_str(&resolve_resp_json).expect("deserialize resolve response");
    assert!(matches!(resolve_resp_parsed, DaemonResponse::ResolveApproval(_)));

    let list_req = DaemonRequest::ListPendingApprovals;
    let list_req_json = serde_json::to_string(&list_req).expect("serialize list request");
    let list_req_parsed: DaemonRequest =
        serde_json::from_str(&list_req_json).expect("deserialize list request");
    assert!(matches!(list_req_parsed, DaemonRequest::ListPendingApprovals));
}

#[test]
fn trace_and_artifact_envelopes_include_conformance_fields() {
    let trace_resp = DaemonResponse::GetTrace(GetTraceResponse {
        trace: TraceSummary {
            trace_id: "trace-1".to_string(),
            task_id: "task-1".to_string(),
            session_id: "session-1".to_string(),
            events: vec![TraceEventSummary {
                event_sequence: 1,
                event_kind: "task_submitted".to_string(),
                details: "goal".to_string(),
            }],
        },
    });
    let trace_json = serde_json::to_string(&trace_resp).expect("serialize trace response");
    let trace_parsed: DaemonResponse = serde_json::from_str(&trace_json).expect("deserialize trace response");
    match trace_parsed {
        DaemonResponse::GetTrace(payload) => {
            assert_eq!(payload.trace.session_id, "session-1");
        }
        other => panic!("unexpected response: {other:?}"),
    }

    let artifacts_resp = DaemonResponse::GetArtifacts(GetArtifactsResponse {
        artifacts: vec![ArtifactSummary {
            artifact_id: "artifact-1".to_string(),
            artifact_kind: "verification_result".to_string(),
            summary: "ok".to_string(),
            produced_by_step_id: "step-1".to_string(),
            produced_by_trace_event_sequence: 3,
        }],
    });
    let artifacts_json = serde_json::to_string(&artifacts_resp).expect("serialize artifacts response");
    let artifacts_parsed: DaemonResponse =
        serde_json::from_str(&artifacts_json).expect("deserialize artifacts response");
    match artifacts_parsed {
        DaemonResponse::GetArtifacts(payload) => {
            assert_eq!(payload.artifacts[0].produced_by_step_id, "step-1");
            assert_eq!(payload.artifacts[0].produced_by_trace_event_sequence, 3);
        }
        other => panic!("unexpected response: {other:?}"),
    }
}
