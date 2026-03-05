use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sharo_core::protocol::{
    ControlTaskRequest, DaemonRequest, DaemonResponse, GetTaskRequest, RegisterSessionRequest, SubmitTaskOpRequest,
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
fn submit_idempotency_key_reuses_task_id() {
    let socket = unique_path("sharo-idempotency", ".sock");
    let store = unique_path("sharo-idempotency", ".json");

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
            session_label: "idempotency".to_string(),
        }),
    ) {
        DaemonResponse::RegisterSession(r) => r.session_id,
        other => panic!("unexpected response: {other:?}"),
    };

    let first = match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_id.clone()),
            goal: "read scope:docs one page".to_string(),
            idempotency_key: Some("idem-key-1".to_string()),
        }),
    ) {
        DaemonResponse::SubmitTask(r) => r,
        other => panic!("unexpected response: {other:?}"),
    };

    let second = match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_id),
            goal: "read scope:docs one page".to_string(),
            idempotency_key: Some("idem-key-1".to_string()),
        }),
    ) {
        DaemonResponse::SubmitTask(r) => r,
        other => panic!("unexpected response: {other:?}"),
    };

    assert_eq!(first.task_id, second.task_id);
    assert!(second.accepted);
    assert_eq!(second.reason.as_deref(), Some("idempotent_replay"));

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
fn replayed_reads_have_no_side_effect() {
    let socket = unique_path("sharo-idem-read", ".sock");
    let store = unique_path("sharo-idem-read", ".json");

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
            session_label: "idem-read".to_string(),
        }),
    ) {
        DaemonResponse::RegisterSession(r) => r.session_id,
        other => panic!("unexpected response: {other:?}"),
    };

    let _ = send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_id.clone()),
            goal: "read scope:docs one page".to_string(),
            idempotency_key: Some("idem-key-read".to_string()),
        }),
    );
    let before = fs::read_to_string(&store).expect("read store before replay");

    let _ = send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_id),
            goal: "read scope:docs one page".to_string(),
            idempotency_key: Some("idem-key-read".to_string()),
        }),
    );
    let after = fs::read_to_string(&store).expect("read store after replay");
    assert_eq!(before, after);

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
fn task_cancel_flow_is_visible_and_durable() {
    let socket = unique_path("sharo-cancel", ".sock");
    let store = unique_path("sharo-cancel", ".json");

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
            session_label: "cancel".to_string(),
        }),
    ) {
        DaemonResponse::RegisterSession(r) => r.session_id,
        other => panic!("unexpected response: {other:?}"),
    };

    let task_id = match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_id),
            goal: "draft scope:notes to cancel".to_string(),
            idempotency_key: Some("idem-cancel".to_string()),
        }),
    ) {
        DaemonResponse::SubmitTask(r) => r.task_id,
        other => panic!("unexpected response: {other:?}"),
    };

    match send_request(
        &socket,
        &DaemonRequest::ControlTask(ControlTaskRequest {
            task_id: task_id.clone(),
            action: "cancel".to_string(),
        }),
    ) {
        DaemonResponse::ControlTask(r) => {
            assert!(r.accepted);
            assert_eq!(r.task_state, "cancelled");
        }
        other => panic!("unexpected response: {other:?}"),
    }

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    if socket.exists() {
        let _ = fs::remove_file(&socket);
    }

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
        .expect("spawn daemon second");

    match send_request(&socket, &DaemonRequest::GetTask(GetTaskRequest { task_id })) {
        DaemonResponse::GetTask(r) => {
            assert_eq!(r.task.task_state, "cancelled");
            assert!(r
                .task
                .blocking_reason
                .as_deref()
                .unwrap_or_default()
                .contains("cancelled_by_operator"));
        }
        other => panic!("unexpected response: {other:?}"),
    }

    daemon2.kill().expect("kill daemon");
    let _ = daemon2.wait();
    if socket.exists() {
        let _ = fs::remove_file(&socket);
    }
    if store.exists() {
        let _ = fs::remove_file(&store);
    }
}
