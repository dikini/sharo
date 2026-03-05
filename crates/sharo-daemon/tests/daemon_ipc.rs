use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sharo_core::protocol::{DaemonRequest, DaemonResponse, SubmitTaskRequest};

fn socket_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("sharo-daemon-test-{}.sock", nanos))
}

#[test]
fn daemon_ipc_submit_roundtrip() {
    let socket = socket_path();

    let mut child = Command::new(env!("CARGO_BIN_EXE_sharo-daemon"))
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket path"),
            "--serve-once",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    let mut connected = None;
    for _ in 0..50 {
        match UnixStream::connect(&socket) {
            Ok(stream) => {
                connected = Some(stream);
                break;
            }
            Err(_) => thread::sleep(Duration::from_millis(20)),
        }
    }

    let mut stream = connected.expect("connect to daemon socket");

    let request = DaemonRequest::Submit(SubmitTaskRequest {
        session_id: Some("session-a".to_string()),
        goal: "read docs".to_string(),
    });
    let payload = serde_json::to_string(&request).expect("serialize request");
    writeln!(stream, "{}", payload).expect("write request");

    let mut line = String::new();
    let mut reader = BufReader::new(stream);
    reader.read_line(&mut line).expect("read response");

    let response: DaemonResponse = serde_json::from_str(line.trim()).expect("parse response");
    match response {
        DaemonResponse::Submit(submit) => {
            assert!(submit.task_id.starts_with("task-"));
        }
        other => panic!("unexpected response: {other:?}"),
    }

    let status = child.wait().expect("wait daemon");
    assert!(status.success());

    if socket.exists() {
        let _ = fs::remove_file(&socket);
    }
}

#[test]
fn daemon_ipc_invalid_json_returns_valid_error_envelope() {
    let socket = socket_path();

    let mut child = Command::new(env!("CARGO_BIN_EXE_sharo-daemon"))
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket path"),
            "--serve-once",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    let mut connected = None;
    for _ in 0..50 {
        match UnixStream::connect(&socket) {
            Ok(stream) => {
                connected = Some(stream);
                break;
            }
            Err(_) => thread::sleep(Duration::from_millis(20)),
        }
    }

    let mut stream = connected.expect("connect to daemon socket");
    writeln!(stream, "{{\"Submit\":{{\"goal\":\"a \\\"quoted\\\" value\"}}")
        .expect("write malformed request");

    let mut line = String::new();
    let mut reader = BufReader::new(stream);
    reader.read_line(&mut line).expect("read response");
    let response: DaemonResponse = serde_json::from_str(line.trim()).expect("parse response");
    match response {
        DaemonResponse::Error { message } => assert!(message.contains("invalid request")),
        other => panic!("unexpected response: {other:?}"),
    }

    let status = child.wait().expect("wait daemon");
    assert!(status.success());
    if socket.exists() {
        let _ = fs::remove_file(&socket);
    }
}

#[test]
fn daemon_ipc_oversized_request_is_rejected() {
    let socket = socket_path();

    let mut child = Command::new(env!("CARGO_BIN_EXE_sharo-daemon"))
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket path"),
            "--serve-once",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    let mut connected = None;
    for _ in 0..50 {
        match UnixStream::connect(&socket) {
            Ok(stream) => {
                connected = Some(stream);
                break;
            }
            Err(_) => thread::sleep(Duration::from_millis(20)),
        }
    }

    let mut stream = connected.expect("connect to daemon socket");
    let oversized = "a".repeat(1_100_000);
    writeln!(stream, "{}", oversized).expect("write oversized request");

    let mut line = String::new();
    let mut reader = BufReader::new(stream);
    reader.read_line(&mut line).expect("read response");
    let response: DaemonResponse = serde_json::from_str(line.trim()).expect("parse response");
    match response {
        DaemonResponse::Error { message } => assert!(message.contains("request_too_large")),
        other => panic!("unexpected response: {other:?}"),
    }

    let status = child.wait().expect("wait daemon");
    assert!(status.success());
    if socket.exists() {
        let _ = fs::remove_file(&socket);
    }
}

#[test]
fn daemon_socket_permissions_are_owner_only() {
    let socket = socket_path();
    let mut child = Command::new(env!("CARGO_BIN_EXE_sharo-daemon"))
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket path"),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    for _ in 0..80 {
        if socket.exists() {
            break;
        }
        thread::sleep(Duration::from_millis(15));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&socket).expect("socket metadata").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    child.kill().expect("kill daemon");
    let _ = child.wait();
    if socket.exists() {
        let _ = fs::remove_file(&socket);
    }
}
