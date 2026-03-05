use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sharo_core::protocol::{
    DaemonRequest, DaemonResponse, GetTaskRequest, ListPendingApprovalsRequest, RegisterSessionRequest,
    ResolveApprovalRequest, SubmitTaskOpRequest,
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

#[test]
fn approval_required_step_waits_then_executes_after_approve() {
    let socket = unique_path("sharo-approval-flow", ".sock");
    let store = unique_path("sharo-approval-flow", ".json");

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
            session_label: "approval-flow".to_string(),
        }),
    ) {
        DaemonResponse::RegisterSession(r) => r.session_id,
        other => panic!("unexpected response: {other:?}"),
    };

    let task_id = match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_id),
            goal: "draft release note".to_string(),
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
            assert_eq!(r.task.task_state, "awaiting_approval");
            assert!(r.task
                .blocking_reason
                .as_deref()
                .unwrap_or_default()
                .contains("approval_required"));
        }
        other => panic!("unexpected response: {other:?}"),
    }

    let approval_id = match send_request(
        &socket,
        &DaemonRequest::ListPendingApprovals(ListPendingApprovalsRequest {}),
    ) {
        DaemonResponse::ListPendingApprovals(r) => {
            assert_eq!(r.approvals.len(), 1);
            assert_eq!(r.approvals[0].task_id, task_id);
            r.approvals[0].approval_id.clone()
        }
        other => panic!("unexpected response: {other:?}"),
    };

    match send_request(
        &socket,
        &DaemonRequest::ResolveApproval(ResolveApprovalRequest {
            approval_id,
            decision: "approve".to_string(),
        }),
    ) {
        DaemonResponse::ResolveApproval(r) => {
            assert_eq!(r.state, "approved");
        }
        other => panic!("unexpected response: {other:?}"),
    }

    match send_request(
        &socket,
        &DaemonRequest::GetTask(GetTaskRequest { task_id }),
    ) {
        DaemonResponse::GetTask(r) => {
            assert_eq!(r.task.task_state, "succeeded");
            assert!(r.task.blocking_reason.is_none());
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
fn approval_resolution_idempotent_by_approval_id() {
    let socket = unique_path("sharo-approval-idempotent", ".sock");
    let store = unique_path("sharo-approval-idempotent", ".json");

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
            session_label: "approval-idempotent".to_string(),
        }),
    ) {
        DaemonResponse::RegisterSession(r) => r.session_id,
        other => panic!("unexpected response: {other:?}"),
    };

    let task_id = match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_id),
            goal: "draft migration plan".to_string(),
            idempotency_key: None,
        }),
    ) {
        DaemonResponse::SubmitTask(r) => r.task_id,
        other => panic!("unexpected response: {other:?}"),
    };

    let approval_id = match send_request(
        &socket,
        &DaemonRequest::ListPendingApprovals(ListPendingApprovalsRequest {}),
    ) {
        DaemonResponse::ListPendingApprovals(r) => r.approvals[0].approval_id.clone(),
        other => panic!("unexpected response: {other:?}"),
    };

    let first = match send_request(
        &socket,
        &DaemonRequest::ResolveApproval(ResolveApprovalRequest {
            approval_id: approval_id.clone(),
            decision: "deny".to_string(),
        }),
    ) {
        DaemonResponse::ResolveApproval(r) => r,
        other => panic!("unexpected response: {other:?}"),
    };
    assert_eq!(first.state, "denied");

    let second = match send_request(
        &socket,
        &DaemonRequest::ResolveApproval(ResolveApprovalRequest {
            approval_id,
            decision: "approve".to_string(),
        }),
    ) {
        DaemonResponse::ResolveApproval(r) => r,
        other => panic!("unexpected response: {other:?}"),
    };
    assert_eq!(second.state, "denied");
    assert_eq!(second.summary, "idempotent_replay");

    match send_request(
        &socket,
        &DaemonRequest::GetTask(GetTaskRequest { task_id }),
    ) {
        DaemonResponse::GetTask(r) => assert_eq!(r.task.task_state, "blocked"),
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
