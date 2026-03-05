use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sharo_core::protocol::{
    DaemonRequest, DaemonResponse, GetTaskRequest, GetTraceRequest, ListPendingApprovalsRequest,
    RegisterSessionRequest, SubmitTaskOpRequest,
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
fn step_terminal_state_is_explicit() {
    let socket = unique_path("sharo-step-terminal", ".sock");
    let store = unique_path("sharo-step-terminal", ".json");

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
            session_label: "step-terminal".to_string(),
        }),
    ) {
        DaemonResponse::RegisterSession(r) => r.session_id,
        other => panic!("unexpected response: {other:?}"),
    };

    let read_task = match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_id.clone()),
            goal: "read scope:docs one page".to_string(),
            idempotency_key: None,
        }),
    ) {
        DaemonResponse::SubmitTask(r) => r.task_id,
        other => panic!("unexpected response: {other:?}"),
    };

    let write_task = match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_id),
            goal: "draft scope:docs one page".to_string(),
            idempotency_key: None,
        }),
    ) {
        DaemonResponse::SubmitTask(r) => r.task_id,
        other => panic!("unexpected response: {other:?}"),
    };

    for task_id in [read_task, write_task] {
        match send_request(&socket, &DaemonRequest::GetTask(GetTaskRequest { task_id })) {
            DaemonResponse::GetTask(r) => {
                let state = r.task.task_state.as_str();
                let allowed = ["succeeded", "awaiting_approval", "blocked", "failed", "cancelled"];
                assert!(allowed.contains(&state), "unexpected explicit state: {state}");
                assert!(!r.task.current_step_summary.is_empty());
            }
            other => panic!("unexpected response: {other:?}"),
        }
    }

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = fs::remove_file(socket);
    let _ = fs::remove_file(store);
}

#[test]
fn trace_continuity_preserved_on_restart() {
    let socket = unique_path("sharo-trace-restart", ".sock");
    let store = unique_path("sharo-trace-restart", ".json");

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
            session_label: "trace-restart".to_string(),
        }),
    ) {
        DaemonResponse::RegisterSession(r) => r.session_id,
        other => panic!("unexpected response: {other:?}"),
    };

    let task_id = match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_id),
            goal: "read scope:docs continuity".to_string(),
            idempotency_key: None,
        }),
    ) {
        DaemonResponse::SubmitTask(r) => r.task_id,
        other => panic!("unexpected response: {other:?}"),
    };

    let before = match send_request(
        &socket,
        &DaemonRequest::GetTrace(GetTraceRequest {
            task_id: task_id.clone(),
        }),
    ) {
        DaemonResponse::GetTrace(r) => r.trace.events,
        other => panic!("unexpected response: {other:?}"),
    };
    assert!(!before.is_empty());

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = fs::remove_file(&socket);

    let mut daemon2 = Command::new(daemon_bin())
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

    let after = match send_request(
        &socket,
        &DaemonRequest::GetTrace(GetTraceRequest { task_id }),
    ) {
        DaemonResponse::GetTrace(r) => r.trace.events,
        other => panic!("unexpected response: {other:?}"),
    };

    assert_eq!(before.len(), after.len());
    for (idx, event) in after.iter().enumerate() {
        assert_eq!(event.event_sequence, (idx as u64) + 1);
    }

    daemon2.kill().expect("kill daemon");
    let _ = daemon2.wait();
    let _ = fs::remove_file(socket);
    let _ = fs::remove_file(store);
}

#[test]
fn recovery_preserves_pending_approval_and_conflict_visibility() {
    let socket = unique_path("sharo-recovery-approval-conflict", ".sock");
    let store = unique_path("sharo-recovery-approval-conflict", ".json");

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
            session_label: "recovery-approval-conflict".to_string(),
        }),
    ) {
        DaemonResponse::RegisterSession(r) => r.session_id,
        other => panic!("unexpected response: {other:?}"),
    };

    let _first_task = match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_id.clone()),
            goal: "draft scope:notes first".to_string(),
            idempotency_key: None,
        }),
    ) {
        DaemonResponse::SubmitTask(r) => r.task_id,
        other => panic!("unexpected response: {other:?}"),
    };

    let second_task = match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_id),
            goal: "draft scope:notes second".to_string(),
            idempotency_key: None,
        }),
    ) {
        DaemonResponse::SubmitTask(r) => r.task_id,
        other => panic!("unexpected response: {other:?}"),
    };

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = fs::remove_file(&socket);

    let mut daemon2 = Command::new(daemon_bin())
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
        &DaemonRequest::ListPendingApprovals(ListPendingApprovalsRequest {}),
    ) {
        DaemonResponse::ListPendingApprovals(r) => assert!(!r.approvals.is_empty()),
        other => panic!("unexpected response: {other:?}"),
    }

    match send_request(
        &socket,
        &DaemonRequest::GetTask(GetTaskRequest {
            task_id: second_task,
        }),
    ) {
        DaemonResponse::GetTask(r) => {
            assert!(r
                .task
                .coordination_summary
                .unwrap_or_default()
                .contains("conflict_id="));
            assert_eq!(r.task.task_state, "awaiting_approval");
        }
        other => panic!("unexpected response: {other:?}"),
    }

    daemon2.kill().expect("kill daemon");
    let _ = daemon2.wait();
    let _ = fs::remove_file(socket);
    let _ = fs::remove_file(store);
}
