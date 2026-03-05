use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sharo_core::protocol::{
    ArtifactSummary, DaemonRequest, DaemonResponse, GetArtifactsRequest, GetTaskRequest,
    GetTraceRequest, RegisterSessionRequest, SubmitTaskOpRequest,
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
            assert_eq!(r.task.task_id, task_id);
            assert_eq!(r.task.task_state, "succeeded");
            assert!(r.task.current_step_summary.contains("read"));
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
