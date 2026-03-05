use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sharo_core::protocol::{
    ArtifactSummary, DaemonRequest, DaemonResponse, GetArtifactsRequest, GetTaskRequest, GetTraceRequest,
    ListPendingApprovalsResponse, RegisterSessionRequest, ResolveApprovalRequest, SubmitTaskOpRequest,
};

fn unique_path(prefix: &str, suffix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("{}-{}{}", prefix, nanos, suffix))
}

fn daemon_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sharo-daemon"))
}

fn send_request(socket: &PathBuf, request: &DaemonRequest) -> DaemonResponse {
    let mut connected = None;
    for _ in 0..80 {
        match UnixStream::connect(socket) {
            Ok(stream) => {
                connected = Some(stream);
                break;
            }
            Err(_) => thread::sleep(Duration::from_millis(15)),
        }
    }

    let mut stream = connected.expect("connect to daemon socket");
    let payload = serde_json::to_string(request).expect("serialize request");
    writeln!(stream, "{}", payload).expect("write request");

    let mut line = String::new();
    let mut reader = BufReader::new(stream);
    reader.read_line(&mut line).expect("read response");
    serde_json::from_str(line.trim()).expect("parse response")
}

fn assert_trace_monotonic(trace: &sharo_core::protocol::TraceSummary) {
    for pair in trace.events.windows(2) {
        assert!(pair[0].event_sequence < pair[1].event_sequence);
    }
}

#[test]
fn scenario_a_read_task_succeeds_with_verification_artifact() {
    let socket = unique_path("sharo-scenario-a", ".sock");
    let store = unique_path("sharo-scenario-a", ".json");

    let mut daemon = Command::new(daemon_bin())
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket path"),
            "--store-path",
            store.to_str().expect("store path"),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    let session_id = match send_request(
        &socket,
        &DaemonRequest::RegisterSession(RegisterSessionRequest {
            session_label: "scenario-a".to_string(),
        }),
    ) {
        DaemonResponse::RegisterSession(r) => r.session_id,
        other => panic!("unexpected response: {other:?}"),
    };

    let task_id = match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_id.clone()),
            goal: "read one context item".to_string(),
            idempotency_key: Some("idem-a-1".to_string()),
        }),
    ) {
        DaemonResponse::SubmitTask(r) => r.task_id,
        other => panic!("unexpected response: {other:?}"),
    };

    match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_id.clone()),
            goal: "read one context item".to_string(),
            idempotency_key: Some("idem-a-1".to_string()),
        }),
    ) {
        DaemonResponse::SubmitTask(r) => assert_eq!(r.task_id, task_id),
        other => panic!("unexpected response: {other:?}"),
    };

    let session_id_2 = match send_request(
        &socket,
        &DaemonRequest::RegisterSession(RegisterSessionRequest {
            session_label: "scenario-a-other-session".to_string(),
        }),
    ) {
        DaemonResponse::RegisterSession(r) => r.session_id,
        other => panic!("unexpected response: {other:?}"),
    };
    match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_id_2),
            goal: "read one context item".to_string(),
            idempotency_key: Some("idem-a-1".to_string()),
        }),
    ) {
        DaemonResponse::SubmitTask(r) => assert_ne!(r.task_id, task_id),
        other => panic!("unexpected response: {other:?}"),
    };

    match send_request(
        &socket,
        &DaemonRequest::GetTask(GetTaskRequest {
            task_id: task_id.clone(),
        }),
    ) {
        DaemonResponse::GetTask(r) => {
            assert_eq!(r.task.task_id, task_id);
            assert_eq!(r.task.task_state, "succeeded");
            assert!(r.task.current_step_summary.contains("read"));
            assert!(r.task.coordination_summary.is_none());
        }
        other => panic!("unexpected response: {other:?}"),
    }

    match send_request(
        &socket,
        &DaemonRequest::GetTrace(GetTraceRequest {
            task_id: task_id.clone(),
        }),
    ) {
        DaemonResponse::GetTrace(r) => {
            assert!(r.trace.events.len() >= 3);
            assert!(r.trace.events.iter().any(|e| e.event_kind == "route_decision"));
            assert!(r.trace.events.iter().any(|e| e.event_kind == "binding_created"));
            assert!(
                r.trace
                    .events
                    .iter()
                    .any(|e| e.event_kind == "binding_redacted_for_model")
            );
            assert_trace_monotonic(&r.trace);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    match send_request(
        &socket,
        &DaemonRequest::GetArtifacts(GetArtifactsRequest {
            task_id: task_id.clone(),
        }),
    ) {
        DaemonResponse::GetArtifacts(r) => {
            let kinds: Vec<&str> = r
                .artifacts
                .iter()
                .map(|a: &ArtifactSummary| a.artifact_kind.as_str())
                .collect();
            assert!(kinds.contains(&"verification_result"));
            assert!(kinds.contains(&"final_result"));
        }
        other => panic!("unexpected response: {other:?}"),
    }

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();

    if socket.exists() {
        let _ = fs::remove_file(&socket);
    }
    if store.exists() {
        let _ = fs::remove_file(&store);
    }
}

#[test]
fn scenario_b_pending_approval_survives_restart_and_can_be_resolved() {
    let socket = unique_path("sharo-scenario-b", ".sock");
    let store = unique_path("sharo-scenario-b", ".json");

    let mut daemon = Command::new(daemon_bin())
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket path"),
            "--store-path",
            store.to_str().expect("store path"),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    let session_id = match send_request(
        &socket,
        &DaemonRequest::RegisterSession(RegisterSessionRequest {
            session_label: "scenario-b".to_string(),
        }),
    ) {
        DaemonResponse::RegisterSession(r) => r.session_id,
        other => panic!("unexpected response: {other:?}"),
    };

    let task_id = match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_id),
            goal: "restricted: write secret".to_string(),
            idempotency_key: None,
        }),
    ) {
        DaemonResponse::SubmitTask(r) => {
            assert_eq!(r.task_state, "awaiting_approval");
            r.task_id
        }
        other => panic!("unexpected response: {other:?}"),
    };

    let approval_id = match send_request(&socket, &DaemonRequest::ListPendingApprovals) {
        DaemonResponse::ListPendingApprovals(ListPendingApprovalsResponse { approvals }) => {
            let p = approvals.iter().find(|a| a.task_id == task_id).expect("approval for task");
            assert_eq!(p.state, "pending");
            p.approval_id.clone()
        }
        other => panic!("unexpected response: {other:?}"),
    };

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    thread::sleep(Duration::from_millis(40));

    let mut daemon = Command::new(daemon_bin())
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket path"),
            "--store-path",
            store.to_str().expect("store path"),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    match send_request(
        &socket,
        &DaemonRequest::GetTask(GetTaskRequest {
            task_id: task_id.clone(),
        }),
    ) {
        DaemonResponse::GetTask(r) => assert_eq!(r.task.task_state, "awaiting_approval"),
        other => panic!("unexpected response: {other:?}"),
    }

    match send_request(
        &socket,
        &DaemonRequest::GetTrace(GetTraceRequest {
            task_id: task_id.clone(),
        }),
    ) {
        DaemonResponse::GetTrace(r) => {
            assert!(r.trace.events.iter().any(|e| {
                e.event_kind == "binding_created" && e.details.contains("visibility=approval_gated")
            }));
            assert!(
                r.trace
                    .events
                    .iter()
                    .any(|e| e.event_kind == "binding_redacted_for_model")
            );
            assert_trace_monotonic(&r.trace);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    match send_request(
        &socket,
        &DaemonRequest::GetArtifacts(GetArtifactsRequest {
            task_id: task_id.clone(),
        }),
    ) {
        DaemonResponse::GetArtifacts(r) => {
            let kinds: Vec<&str> = r
                .artifacts
                .iter()
                .map(|a: &ArtifactSummary| a.artifact_kind.as_str())
                .collect();
            assert!(!kinds.contains(&"final_result"));
            assert!(kinds.contains(&"verification_result"));
        }
        other => panic!("unexpected response: {other:?}"),
    }

    match send_request(
        &socket,
        &DaemonRequest::ResolveApproval(ResolveApprovalRequest {
            approval_id,
            decision: "approve".to_string(),
        }),
    ) {
        DaemonResponse::ResolveApproval(r) => assert_eq!(r.state, "approved"),
        other => panic!("unexpected response: {other:?}"),
    }

    match send_request(
        &socket,
        &DaemonRequest::ResolveApproval(ResolveApprovalRequest {
            approval_id: "approval-999999".to_string(),
            decision: "approved".to_string(),
        }),
    ) {
        DaemonResponse::Error { message } => assert!(message.contains("approval_decision_invalid")),
        other => panic!("unexpected response: {other:?}"),
    }

    match send_request(
        &socket,
        &DaemonRequest::GetTask(GetTaskRequest {
            task_id: task_id.clone(),
        }),
    ) {
        DaemonResponse::GetTask(r) => assert_eq!(r.task.task_state, "succeeded"),
        other => panic!("unexpected response: {other:?}"),
    }

    match send_request(
        &socket,
        &DaemonRequest::GetArtifacts(GetArtifactsRequest {
            task_id: task_id.clone(),
        }),
    ) {
        DaemonResponse::GetArtifacts(r) => {
            let kinds: Vec<&str> = r
                .artifacts
                .iter()
                .map(|a: &ArtifactSummary| a.artifact_kind.as_str())
                .collect();
            assert!(kinds.contains(&"final_result"));
        }
        other => panic!("unexpected response: {other:?}"),
    }

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
}

#[test]
fn scenario_c_overlap_visibility_survives_restart() {
    let socket = unique_path("sharo-scenario-c", ".sock");
    let store = unique_path("sharo-scenario-c", ".json");

    let mut daemon = Command::new(daemon_bin())
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket path"),
            "--store-path",
            store.to_str().expect("store path"),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    let session_1 = match send_request(
        &socket,
        &DaemonRequest::RegisterSession(RegisterSessionRequest {
            session_label: "s1".to_string(),
        }),
    ) {
        DaemonResponse::RegisterSession(r) => r.session_id,
        other => panic!("unexpected response: {other:?}"),
    };
    let session_2 = match send_request(
        &socket,
        &DaemonRequest::RegisterSession(RegisterSessionRequest {
            session_label: "s2".to_string(),
        }),
    ) {
        DaemonResponse::RegisterSession(r) => r.session_id,
        other => panic!("unexpected response: {other:?}"),
    };

    let _ = send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_1),
            goal: "resource:alpha overlap check".to_string(),
            idempotency_key: None,
        }),
    );

    let task_2 = match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_2),
            goal: "resource:alpha overlap check".to_string(),
            idempotency_key: None,
        }),
    ) {
        DaemonResponse::SubmitTask(r) => r.task_id,
        other => panic!("unexpected response: {other:?}"),
    };

    match send_request(
        &socket,
        &DaemonRequest::GetTask(GetTaskRequest {
            task_id: task_2.clone(),
        }),
    ) {
        DaemonResponse::GetTask(r) => {
            let summary = r.task.coordination_summary.unwrap_or_default();
            assert!(summary.contains("conflict"));
        }
        other => panic!("unexpected response: {other:?}"),
    }

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    thread::sleep(Duration::from_millis(40));

    let mut daemon = Command::new(daemon_bin())
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket path"),
            "--store-path",
            store.to_str().expect("store path"),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    match send_request(
        &socket,
        &DaemonRequest::GetTask(GetTaskRequest {
            task_id: task_2.clone(),
        }),
    ) {
        DaemonResponse::GetTask(r) => {
            let summary = r.task.coordination_summary.unwrap_or_default();
            assert!(summary.contains("conflict"));
        }
        other => panic!("unexpected response: {other:?}"),
    }

    match send_request(
        &socket,
        &DaemonRequest::GetTrace(GetTraceRequest {
            task_id: task_2.clone(),
        }),
    ) {
        DaemonResponse::GetTrace(r) => {
            assert!(r.trace.events.iter().any(|e| e.event_kind == "conflict_detected"));
            assert_trace_monotonic(&r.trace);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
}

#[test]
fn invalid_manifest_is_blocked_with_explicit_reason() {
    let socket = unique_path("sharo-scenario-manifest", ".sock");
    let store = unique_path("sharo-scenario-manifest", ".json");

    let mut daemon = Command::new(daemon_bin())
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket path"),
            "--store-path",
            store.to_str().expect("store path"),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    let session_id = match send_request(
        &socket,
        &DaemonRequest::RegisterSession(RegisterSessionRequest {
            session_label: "scenario-manifest".to_string(),
        }),
    ) {
        DaemonResponse::RegisterSession(r) => r.session_id,
        other => panic!("unexpected response: {other:?}"),
    };

    let task_id = match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_id),
            goal: "invalid_manifest:missing-capability".to_string(),
            idempotency_key: None,
        }),
    ) {
        DaemonResponse::SubmitTask(r) => r.task_id,
        other => panic!("unexpected response: {other:?}"),
    };

    match send_request(
        &socket,
        &DaemonRequest::GetTask(GetTaskRequest {
            task_id: task_id.clone(),
        }),
    ) {
        DaemonResponse::GetTask(r) => {
            assert_eq!(r.task.task_state, "blocked");
            let reason = r.task.blocking_reason.unwrap_or_default();
            assert!(reason.contains("manifest_invalid"));
        }
        other => panic!("unexpected response: {other:?}"),
    }

    match send_request(
        &socket,
        &DaemonRequest::GetArtifacts(GetArtifactsRequest {
            task_id: task_id.clone(),
        }),
    ) {
        DaemonResponse::GetArtifacts(r) => {
            let kinds: Vec<&str> = r
                .artifacts
                .iter()
                .map(|a: &ArtifactSummary| a.artifact_kind.as_str())
                .collect();
            assert!(!kinds.contains(&"final_result"));
            assert!(kinds.contains(&"failure_record"));
        }
        other => panic!("unexpected response: {other:?}"),
    }

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
}

#[test]
fn store_file_permissions_are_restricted() {
    let socket = unique_path("sharo-store-perms", ".sock");
    let store = unique_path("sharo-store-perms", ".json");

    let mut daemon = Command::new(daemon_bin())
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket path"),
            "--store-path",
            store.to_str().expect("store path"),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    let _ = send_request(
        &socket,
        &DaemonRequest::RegisterSession(RegisterSessionRequest {
            session_label: "perms".to_string(),
        }),
    );
    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = fs::remove_file(&socket);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&store, fs::Permissions::from_mode(0o644))
            .expect("set permissive perms");
    }

    let mut daemon = Command::new(daemon_bin())
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket path"),
            "--store-path",
            store.to_str().expect("store path"),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    let _ = send_request(
        &socket,
        &DaemonRequest::RegisterSession(RegisterSessionRequest {
            session_label: "perms-2".to_string(),
        }),
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&store).expect("store metadata").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
}
