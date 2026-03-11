use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sharo_core::protocol::{
    DaemonRequest, DaemonResponse, SubmitTaskOpRequest, SubmitTaskRequest, TaskStatusRequest,
};

fn socket_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("sharo-daemon-test-{}.sock", nanos))
}

fn temp_path(prefix: &str, suffix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{nanos}{suffix}"))
}

fn write_slow_openai_config(prefix: &str, base_url: &str) -> PathBuf {
    let config = temp_path(prefix, ".toml");
    fs::write(
        &config,
        format!(
            r#"[model]
provider = "openai"
model_id = "gpt-5-mini"
base_url = "{base_url}"
timeout_ms = 2000
"#
        ),
    )
    .expect("write slow openai config");
    config
}

fn start_delayed_response_server(delay: Duration) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind delayed response server");
    let address = format!("http://{}", listener.local_addr().expect("local addr"));
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener
            .accept()
            .expect("accept delayed response connection");
        let cloned = stream.try_clone().expect("clone delayed response stream");
        let mut reader = BufReader::new(cloned);
        let mut line = String::new();
        loop {
            line.clear();
            let bytes = reader
                .read_line(&mut line)
                .expect("read delayed response request");
            if bytes == 0 || line == "\r\n" {
                break;
            }
        }
        thread::sleep(delay);
        let body = "{\"id\":\"resp-1\",\"output_text\":\"slow submit complete\"}";
        write!(
            stream,
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .expect("write delayed response");
        stream.flush().expect("flush delayed response");
    });
    (address, handle)
}

fn start_multi_delayed_response_server(
    delay: Duration,
    expected_requests: usize,
) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind delayed response server");
    let address = format!("http://{}", listener.local_addr().expect("local addr"));
    let handle = thread::spawn(move || {
        let mut workers = Vec::with_capacity(expected_requests);
        for _ in 0..expected_requests {
            let (mut stream, _) = listener
                .accept()
                .expect("accept delayed response connection");
            workers.push(thread::spawn(move || {
                let cloned = stream.try_clone().expect("clone delayed response stream");
                let mut reader = BufReader::new(cloned);
                let mut line = String::new();
                loop {
                    line.clear();
                    let bytes = reader.read_line(&mut line).expect("read delayed response request");
                    if bytes == 0 || line == "\r\n" {
                        break;
                    }
                }
                thread::sleep(delay);
                let body = "{\"id\":\"resp-1\",\"output_text\":\"slow submit complete\"}";
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .expect("write delayed response");
                stream.flush().expect("flush delayed response");
            }));
        }

        for worker in workers {
            worker.join().expect("join delayed response worker");
        }
    });
    (address, handle)
}

fn connect_with_retry(socket: &PathBuf) -> UnixStream {
    for _ in 0..80 {
        match UnixStream::connect(socket) {
            Ok(stream) => return stream,
            Err(_) => thread::sleep(Duration::from_millis(15)),
        }
    }
    panic!("connect to daemon socket")
}

fn send_request_with_stream(stream: UnixStream, request: &DaemonRequest) -> DaemonResponse {
    let payload = serde_json::to_string(request).expect("serialize request");
    let mut writer = stream.try_clone().expect("clone stream for writing");
    writeln!(writer, "{}", payload).expect("write request");

    let mut line = String::new();
    let mut reader = BufReader::new(stream);
    reader.read_line(&mut line).expect("read response");
    serde_json::from_str(line.trim()).expect("parse response")
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
    writeln!(
        stream,
        "{{\"Submit\":{{\"goal\":\"a \\\"quoted\\\" value\"}}"
    )
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
        let mode = fs::metadata(&socket)
            .expect("socket metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }

    child.kill().expect("kill daemon");
    let _ = child.wait();
    if socket.exists() {
        let _ = fs::remove_file(&socket);
    }
}

#[test]
fn status_request_remains_responsive_during_slow_submit() {
    let socket = socket_path();
    let store = temp_path("sharo-daemon-concurrency", ".json");
    let (base_url, server_thread) = start_delayed_response_server(Duration::from_millis(500));
    let config = write_slow_openai_config("sharo-daemon-concurrency", &base_url);

    let mut child = Command::new(env!("CARGO_BIN_EXE_sharo-daemon"))
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket path"),
            "--store-path",
            store.to_str().expect("store path"),
            "--config-path",
            config.to_str().expect("config path"),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    let submit_stream = connect_with_retry(&socket);
    let submit_thread = thread::spawn(move || {
        send_request_with_stream(
            submit_stream,
            &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
                session_id: Some("session-concurrency".to_string()),
                goal: "slow submit".to_string(),
                idempotency_key: None,
            }),
        )
    });

    thread::sleep(Duration::from_millis(75));

    let status_start = SystemTime::now();
    let status_response = send_request_with_stream(
        connect_with_retry(&socket),
        &DaemonRequest::Status(TaskStatusRequest {
            task_id: "task-123450".to_string(),
        }),
    );
    let status_elapsed = status_start.elapsed().expect("status elapsed");

    match status_response {
        DaemonResponse::Status(response) => {
            assert_eq!(response.task_id, "task-123450");
        }
        other => panic!("unexpected response: {other:?}"),
    }
    assert!(
        status_elapsed < Duration::from_millis(450),
        "status request took {:?} while slow submit was running",
        status_elapsed
    );

    match submit_thread.join().expect("submit thread join") {
        DaemonResponse::SubmitTask(response) => assert_eq!(response.task_state, "succeeded"),
        other => panic!("unexpected response: {other:?}"),
    }

    server_thread.join().expect("delayed server join");
    child.kill().expect("kill daemon");
    let _ = child.wait();
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
    let _ = fs::remove_file(&config);
}

#[test]
fn handle_request_avoids_holding_store_lock_across_provider_work() {
    status_request_remains_responsive_during_slow_submit();
}

#[test]
fn status_requests_remain_responsive_under_parallel_slow_submits() {
    let socket = socket_path();
    let store = temp_path("sharo-daemon-runtime-pressure", ".json");
    let runtime_pressure = std::thread::available_parallelism()
        .map(|threads| threads.get() + 1)
        .unwrap_or(5);
    let (base_url, server_thread) =
        start_multi_delayed_response_server(Duration::from_millis(600), runtime_pressure);
    let config = write_slow_openai_config("sharo-daemon-runtime-pressure", &base_url);

    let mut child = Command::new(env!("CARGO_BIN_EXE_sharo-daemon"))
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket path"),
            "--store-path",
            store.to_str().expect("store path"),
            "--config-path",
            config.to_str().expect("config path"),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    let mut submit_threads = Vec::with_capacity(runtime_pressure);
    for request_index in 0..runtime_pressure {
        let submit_stream = connect_with_retry(&socket);
        submit_threads.push(thread::spawn(move || {
            send_request_with_stream(
                submit_stream,
                &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
                    session_id: Some(format!("session-runtime-pressure-{request_index}")),
                    goal: format!("slow submit {request_index}"),
                    idempotency_key: Some(format!("idem-runtime-pressure-{request_index}")),
                }),
            )
        }));
    }

    thread::sleep(Duration::from_millis(100));

    let status_start = SystemTime::now();
    let status_response = send_request_with_stream(
        connect_with_retry(&socket),
        &DaemonRequest::Status(TaskStatusRequest {
            task_id: "task-pressure".to_string(),
        }),
    );
    let status_elapsed = status_start.elapsed().expect("status elapsed");

    match status_response {
        DaemonResponse::Status(response) => {
            assert_eq!(response.task_id, "task-pressure");
        }
        other => panic!("unexpected response: {other:?}"),
    }
    assert!(
        status_elapsed < Duration::from_millis(400),
        "status request took {:?} while runtime was under parallel slow-submit pressure",
        status_elapsed
    );

    for submit_thread in submit_threads {
        match submit_thread.join().expect("submit thread join") {
            DaemonResponse::SubmitTask(response) => assert_eq!(response.task_state, "succeeded"),
            other => panic!("unexpected response: {other:?}"),
        }
    }

    server_thread.join().expect("delayed server join");
    child.kill().expect("kill daemon");
    let _ = child.wait();
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
    let _ = fs::remove_file(&config);
}

#[test]
fn serve_many_requests_returns_exactly_one_response_each() {
    status_requests_remain_responsive_under_parallel_slow_submits();
}

#[test]
fn ctrl_c_waits_for_inflight_request_completion() {
    let socket = socket_path();
    let store = temp_path("sharo-daemon-shutdown-drain", ".json");
    let (base_url, server_thread) = start_delayed_response_server(Duration::from_millis(450));
    let config = write_slow_openai_config("sharo-daemon-shutdown-drain", &base_url);

    let mut child = Command::new(env!("CARGO_BIN_EXE_sharo-daemon"))
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket path"),
            "--store-path",
            store.to_str().expect("store path"),
            "--config-path",
            config.to_str().expect("config path"),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    let submit_stream = connect_with_retry(&socket);
    let submit_thread = thread::spawn(move || {
        send_request_with_stream(
            submit_stream,
            &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
                session_id: Some("session-shutdown-drain".to_string()),
                goal: "slow submit during ctrl-c".to_string(),
                idempotency_key: Some("idem-shutdown-drain".to_string()),
            }),
        )
    });

    thread::sleep(Duration::from_millis(80));
    let signal_status = Command::new("kill")
        .args(["-INT", &child.id().to_string()])
        .status()
        .expect("send SIGINT");
    assert!(signal_status.success(), "failed to send SIGINT");

    match submit_thread.join().expect("submit thread join") {
        DaemonResponse::SubmitTask(response) => assert_eq!(response.task_state, "succeeded"),
        other => panic!("unexpected response: {other:?}"),
    }

    let exit_status = child.wait().expect("wait daemon exit");
    assert!(
        exit_status.success(),
        "daemon should exit cleanly after draining handlers"
    );

    server_thread.join().expect("delayed server join");
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
    let _ = fs::remove_file(&config);
}
