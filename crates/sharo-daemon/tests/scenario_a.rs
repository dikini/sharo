use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
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

fn write_deterministic_config(prefix: &str) -> PathBuf {
    let config = unique_path(prefix, ".toml");
    fs::write(
        &config,
        r#"[model]
provider = "deterministic"
model_id = "mock"
timeout_ms = 1000
"#,
    )
    .expect("write deterministic config");
    config
}

fn write_reasoning_pressure_config(prefix: &str) -> PathBuf {
    let config = unique_path(prefix, ".toml");
    fs::write(
        &config,
        r#"[model]
provider = "deterministic"
model_id = "mock"
timeout_ms = 1000

[reasoning_policy]
max_prompt_chars = 10000
max_memory_lines = 1
forbidden_runtime_fields = ["secret"]

[reasoning_context]
system = "system=keep-safe"
persona = "verbosity=high"
memory = """m1
m2
m3 with many words for compression pressure"""
runtime = "secret=abc123"
"#,
    )
    .expect("write pressure config");
    config
}

fn write_reasoning_failure_config(prefix: &str) -> PathBuf {
    let config = unique_path(prefix, ".toml");
    fs::write(
        &config,
        r#"[model]
provider = "deterministic"
model_id = "mock"
timeout_ms = 1000

[reasoning_policy]
max_prompt_chars = 1
"#,
    )
    .expect("write failure config");
    config
}

fn write_openai_missing_auth_config(prefix: &str) -> PathBuf {
    let config = unique_path(prefix, ".toml");
    fs::write(
        &config,
        r#"[model]
provider = "openai"
model_id = "gpt-5-mini"
base_url = "https://api.openai.com"
auth_env_key = "SHARO_TEST_MISSING_OPENAI_KEY"
timeout_ms = 1000
"#,
    )
    .expect("write openai auth config");
    config
}

fn write_slow_openai_config(prefix: &str, base_url: &str) -> PathBuf {
    let config = unique_path(prefix, ".toml");
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

fn write_bounded_openai_config(
    prefix: &str,
    base_url: &str,
    min_threads: usize,
    max_threads: usize,
) -> PathBuf {
    let config = unique_path(prefix, ".toml");
    fs::write(
        &config,
        format!(
            r#"[model]
provider = "openai"
model_id = "gpt-5-mini"
base_url = "{base_url}"
timeout_ms = 2000

[connector_pool]
min_threads = {min_threads}
max_threads = {max_threads}
queue_capacity = 16
scale_up_queue_threshold = 1
scale_down_idle_ms = 5000
cooldown_ms = 1
"#
        ),
    )
    .expect("write bounded openai config");
    config
}

fn start_delayed_response_server(delay: Duration) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind delayed response server");
    let address = format!("http://{}", listener.local_addr().expect("local addr"));
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept delayed response connection");
        let cloned = stream.try_clone().expect("clone delayed response stream");
        let mut reader = BufReader::new(cloned);
        let mut line = String::new();
        loop {
            line.clear();
            let bytes = reader.read_line(&mut line).expect("read delayed request");
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

fn start_status_response_server(status_line: &str) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind status response server");
    let address = format!("http://{}", listener.local_addr().expect("local addr"));
    let status_line = status_line.to_string();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept status response connection");
        let cloned = stream.try_clone().expect("clone status response stream");
        let mut reader = BufReader::new(cloned);
        let mut line = String::new();
        loop {
            line.clear();
            let bytes = reader.read_line(&mut line).expect("read status response request");
            if bytes == 0 || line == "\r\n" {
                break;
            }
        }

        let body = "{\"error\":\"simulated\"}";
        write!(
            stream,
            "HTTP/1.1 {status_line}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .expect("write status response");
        stream.flush().expect("flush status response");
    });
    (address, handle)
}

fn update_max_observed(max_observed: &AtomicUsize, candidate: usize) {
    let mut current = max_observed.load(Ordering::SeqCst);
    while candidate > current {
        match max_observed.compare_exchange_weak(
            current,
            candidate,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => return,
            Err(observed) => current = observed,
        }
    }
}

fn start_counting_response_server(
    delay: Duration,
    expected_requests: usize,
) -> (String, Arc<AtomicUsize>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind counting response server");
    let address = format!("http://{}", listener.local_addr().expect("local addr"));
    let active = Arc::new(AtomicUsize::new(0));
    let max_observed = Arc::new(AtomicUsize::new(0));
    let max_observed_for_thread = Arc::clone(&max_observed);
    let active_for_thread = Arc::clone(&active);
    let handle = thread::spawn(move || {
        let mut workers = Vec::new();
        for _ in 0..expected_requests {
            let (mut stream, _) = listener.accept().expect("accept counting response connection");
            let active = Arc::clone(&active_for_thread);
            let max_observed = Arc::clone(&max_observed_for_thread);
            workers.push(thread::spawn(move || {
                let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                update_max_observed(&max_observed, current);

                let cloned = stream.try_clone().expect("clone counting response stream");
                let mut reader = BufReader::new(cloned);
                let mut line = String::new();
                loop {
                    line.clear();
                    let bytes = reader.read_line(&mut line).expect("read counting request");
                    if bytes == 0 || line == "\r\n" {
                        break;
                    }
                }

                thread::sleep(delay);
                let body = "{\"id\":\"resp-burst\",\"output_text\":\"burst submit complete\"}";
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .expect("write counting response");
                stream.flush().expect("flush counting response");
                active.fetch_sub(1, Ordering::SeqCst);
            }));
        }

        for worker in workers {
            worker.join().expect("join counting response worker");
        }
    });

    (address, max_observed, handle)
}

fn send_request_on_stream(stream: UnixStream, request: &DaemonRequest) -> DaemonResponse {
    let payload = serde_json::to_string(request).expect("serialize request");
    let mut writer = stream.try_clone().expect("clone stream for writing");
    writeln!(writer, "{}", payload).expect("write request");

    let mut line = String::new();
    let mut reader = BufReader::new(stream);
    reader.read_line(&mut line).expect("read response");
    serde_json::from_str(line.trim()).expect("parse response")
}

fn send_request(socket: &PathBuf, request: &DaemonRequest) -> DaemonResponse {
    for _ in 0..5 {
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
        if line.trim().is_empty() {
            thread::sleep(Duration::from_millis(20));
            continue;
        }
        if let Ok(parsed) = serde_json::from_str(line.trim()) {
            return parsed;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("parse response")
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
    let config = write_deterministic_config("sharo-scenario-a");

    let mut daemon = Command::new(daemon_bin())
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
            let preview = r.task.result_preview.unwrap_or_default();
            assert!(preview.contains("deterministic-response"));
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
            assert_eq!(r.trace.session_id, session_id);
            assert!(r.trace.events.iter().any(|e| e.event_kind == "route_decision"));
            assert!(r.trace.events.iter().any(|e| e.event_kind == "fit_loop_fitted"));
            assert!(
                r.trace.events
                    .iter()
                    .any(|e| e.event_kind == "model_output_received" && e.details.contains("deterministic-response"))
            );
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
            assert!(kinds.contains(&"fit_loop_decision"));
            assert!(kinds.contains(&"model_output"));
            assert!(kinds.contains(&"verification_result"));
            assert!(kinds.contains(&"final_result"));
            assert!(
                r.artifacts.iter().any(|a| {
                    a.artifact_kind == "model_output"
                        && a.summary.contains("deterministic-response")
                })
            );
            for artifact in &r.artifacts {
                assert!(!artifact.produced_by_step_id.is_empty());
                assert!(artifact.produced_by_trace_event_sequence > 0);
            }
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
    if config.exists() {
        let _ = fs::remove_file(&config);
    }
}

#[test]
fn idempotent_retry_after_save_failure_creates_one_committed_task() {
    let socket = unique_path("sharo-scenario-a-save-retry", ".sock");
    let store_dir = unique_path("sharo-scenario-a-save-retry", ".d");
    fs::create_dir_all(&store_dir).expect("create store dir");
    let store = store_dir.join("daemon-store.json");
    let config = write_deterministic_config("sharo-scenario-a-save-retry");

    let mut daemon = Command::new(daemon_bin())
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

    fs::remove_dir_all(&store_dir).expect("remove store dir before first submit");

    match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some("session-save-retry".to_string()),
            goal: "read one context item".to_string(),
            idempotency_key: Some("idem-save-retry".to_string()),
        }),
    ) {
        DaemonResponse::Error { message } => {
            assert!(
                message.contains("store_parent_missing") || message.contains("store_open_failed"),
                "unexpected save error: {message}"
            );
        }
        other => panic!("unexpected response: {other:?}"),
    }

    fs::create_dir_all(&store_dir).expect("recreate store dir");

    let task_id = match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some("session-save-retry".to_string()),
            goal: "read one context item".to_string(),
            idempotency_key: Some("idem-save-retry".to_string()),
        }),
    ) {
        DaemonResponse::SubmitTask(response) => response.task_id,
        other => panic!("unexpected response: {other:?}"),
    };

    match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some("session-save-retry".to_string()),
            goal: "read one context item".to_string(),
            idempotency_key: Some("idem-save-retry".to_string()),
        }),
    ) {
        DaemonResponse::SubmitTask(response) => assert_eq!(response.task_id, task_id),
        other => panic!("unexpected response: {other:?}"),
    }

    let store_json = fs::read_to_string(&store).expect("read store");
    let persisted: serde_json::Value = serde_json::from_str(&store_json).expect("parse store");
    let task_count = persisted["tasks"]
        .as_object()
        .map(|tasks| tasks.len())
        .expect("tasks map");
    assert_eq!(task_count, 1, "expected exactly one committed task");

    match send_request(
        &socket,
        &DaemonRequest::GetTask(GetTaskRequest { task_id }),
    ) {
        DaemonResponse::GetTask(response) => {
            assert_eq!(response.task.session_id, "session-save-retry");
            assert_eq!(response.task.task_state, "succeeded");
        }
        other => panic!("unexpected response: {other:?}"),
    }

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&config);
    let _ = fs::remove_file(&store);
    let _ = fs::remove_dir_all(&store_dir);
}

#[test]
fn daemon_burst_submit_never_exceeds_configured_connector_workers() {
    let socket = unique_path("sharo-scenario-a-burst", ".sock");
    let store = unique_path("sharo-scenario-a-burst", ".json");
    let burst_count = 6;
    let max_threads = 2;
    let (base_url, max_observed, server_handle) =
        start_counting_response_server(Duration::from_millis(120), burst_count);
    let config =
        write_bounded_openai_config("sharo-scenario-a-burst", &base_url, 1, max_threads);

    let mut daemon = Command::new(daemon_bin())
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

    let mut workers = Vec::new();
    for request_index in 0..burst_count {
        let socket = socket.clone();
        workers.push(thread::spawn(move || {
            match send_request(
                &socket,
                &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
                    session_id: Some("session-burst".to_string()),
                    goal: format!("read one context item burst-{request_index}"),
                    idempotency_key: Some(format!("idem-burst-{request_index}")),
                }),
            ) {
                DaemonResponse::SubmitTask(response) => response,
                other => panic!("unexpected response: {other:?}"),
            }
        }));
    }

    for worker in workers {
        let response = worker.join().expect("join submit worker");
        assert_eq!(response.task_state, "succeeded");
    }

    server_handle.join().expect("join counting server");
    assert!(
        max_observed.load(Ordering::SeqCst) <= max_threads,
        "observed {} concurrent upstream requests with max_threads={max_threads}",
        max_observed.load(Ordering::SeqCst)
    );

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
    let _ = fs::remove_file(&config);
}

#[test]
fn scenario_a_success_survives_restart_with_same_trace_and_preview() {
    let socket = unique_path("sharo-scenario-a-restart", ".sock");
    let store = unique_path("sharo-scenario-a-restart", ".json");
    let config = write_deterministic_config("sharo-scenario-a-restart");

    let mut daemon = Command::new(daemon_bin())
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

    let session_id = match send_request(
        &socket,
        &DaemonRequest::RegisterSession(RegisterSessionRequest {
            session_label: "scenario-a-restart".to_string(),
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
            idempotency_key: Some("idem-a-restart".to_string()),
        }),
    ) {
        DaemonResponse::SubmitTask(r) => r.task_id,
        other => panic!("unexpected response: {other:?}"),
    };

    let before_task = match send_request(
        &socket,
        &DaemonRequest::GetTask(GetTaskRequest {
            task_id: task_id.clone(),
        }),
    ) {
        DaemonResponse::GetTask(r) => r.task,
        other => panic!("unexpected response: {other:?}"),
    };
    let before_trace = match send_request(
        &socket,
        &DaemonRequest::GetTrace(GetTraceRequest {
            task_id: task_id.clone(),
        }),
    ) {
        DaemonResponse::GetTrace(r) => r.trace,
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
            "--config-path",
            config.to_str().expect("config path"),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    let after_task = match send_request(
        &socket,
        &DaemonRequest::GetTask(GetTaskRequest {
            task_id: task_id.clone(),
        }),
    ) {
        DaemonResponse::GetTask(r) => r.task,
        other => panic!("unexpected response: {other:?}"),
    };
    let after_trace = match send_request(
        &socket,
        &DaemonRequest::GetTrace(GetTraceRequest {
            task_id: task_id.clone(),
        }),
    ) {
        DaemonResponse::GetTrace(r) => r.trace,
        other => panic!("unexpected response: {other:?}"),
    };

    assert_eq!(before_task.task_id, after_task.task_id);
    assert_eq!(before_task.session_id, after_task.session_id);
    assert_eq!(before_task.task_state, "succeeded");
    assert_eq!(after_task.task_state, "succeeded");
    assert_eq!(before_task.result_preview, after_task.result_preview);
    assert!(
        after_task
            .result_preview
            .as_deref()
            .unwrap_or_default()
            .contains("deterministic-response")
    );

    assert_eq!(before_trace.trace_id, after_trace.trace_id);
    assert_eq!(before_trace.task_id, after_trace.task_id);
    assert_eq!(before_trace.session_id, after_trace.session_id);
    assert_eq!(before_trace.events, after_trace.events);
    assert_trace_monotonic(&after_trace);

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
    let _ = fs::remove_file(&config);
}

#[test]
fn scenario_b_pending_approval_survives_restart_and_can_be_resolved() {
    let socket = unique_path("sharo-scenario-b", ".sock");
    let store = unique_path("sharo-scenario-b", ".json");
    let config = write_deterministic_config("sharo-scenario-b");

    let mut daemon = Command::new(daemon_bin())
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
    thread::sleep(Duration::from_millis(60));
    assert!(daemon.try_wait().expect("daemon status check").is_none());

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
            "--config-path",
            config.to_str().expect("config path"),
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
        DaemonResponse::GetTask(r) => {
            assert_eq!(r.task.task_state, "awaiting_approval");
            assert!(r.task.result_preview.is_none());
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
            assert_eq!(r.trace.session_id, "session-000001");
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
            for artifact in &r.artifacts {
                assert!(!artifact.produced_by_step_id.is_empty());
                assert!(artifact.produced_by_trace_event_sequence > 0);
            }
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
        DaemonResponse::GetTask(r) => {
            assert_eq!(r.task.task_state, "succeeded");
            let preview = r.task.result_preview.unwrap_or_default();
            assert!(preview.contains("deterministic-response"));
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
            assert!(kinds.contains(&"final_result"));
            for artifact in &r.artifacts {
                assert!(!artifact.produced_by_step_id.is_empty());
                assert!(artifact.produced_by_trace_event_sequence > 0);
            }
        }
        other => panic!("unexpected response: {other:?}"),
    }

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
    let _ = fs::remove_file(&config);
}

#[test]
fn scenario_b_denied_approval_blocks_without_success_records() {
    let socket = unique_path("sharo-scenario-b-deny", ".sock");
    let store = unique_path("sharo-scenario-b-deny", ".json");
    let config = write_deterministic_config("sharo-scenario-b-deny");

    let mut daemon = Command::new(daemon_bin())
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

    let session_id = match send_request(
        &socket,
        &DaemonRequest::RegisterSession(RegisterSessionRequest {
            session_label: "scenario-b-deny".to_string(),
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
            approvals
                .iter()
                .find(|a| a.task_id == task_id)
                .expect("approval for task")
                .approval_id
                .clone()
        }
        other => panic!("unexpected response: {other:?}"),
    };

    match send_request(
        &socket,
        &DaemonRequest::ResolveApproval(ResolveApprovalRequest {
            approval_id: approval_id.clone(),
            decision: "deny".to_string(),
        }),
    ) {
        DaemonResponse::ResolveApproval(r) => assert_eq!(r.state, "denied"),
        other => panic!("unexpected response: {other:?}"),
    }

    match send_request(
        &socket,
        &DaemonRequest::ResolveApproval(ResolveApprovalRequest {
            approval_id,
            decision: "deny".to_string(),
        }),
    ) {
        DaemonResponse::ResolveApproval(r) => assert_eq!(r.state, "denied"),
        other => panic!("unexpected response: {other:?}"),
    }

    match send_request(
        &socket,
        &DaemonRequest::GetTask(GetTaskRequest {
            task_id: task_id.clone(),
        }),
    ) {
        DaemonResponse::GetTask(r) => {
            assert_eq!(r.task.task_state, "blocked");
            assert_eq!(r.task.blocking_reason.as_deref(), Some("approval_denied"));
            assert!(r.task.result_preview.is_none());
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
            let last = r.trace.events.last().expect("trace event");
            assert_eq!(last.event_kind, "approval_resolved");
            assert_eq!(last.details, "denied");
            assert!(!r.trace.events.iter().any(|e| e.event_kind == "task_succeeded"));
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

    match send_request(&socket, &DaemonRequest::ListPendingApprovals) {
        DaemonResponse::ListPendingApprovals(ListPendingApprovalsResponse { approvals }) => {
            assert!(!approvals.iter().any(|a| a.task_id == task_id));
        }
        other => panic!("unexpected response: {other:?}"),
    }

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
    let _ = fs::remove_file(&config);
}

#[test]
fn scenario_s2_fit_loop_adjustment_is_visible_in_runtime_records() {
    let socket = unique_path("sharo-scenario-s2", ".sock");
    let store = unique_path("sharo-scenario-s2", ".json");
    let config = write_reasoning_pressure_config("sharo-scenario-s2");

    let mut daemon = Command::new(daemon_bin())
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

    let session_id = match send_request(
        &socket,
        &DaemonRequest::RegisterSession(RegisterSessionRequest {
            session_label: "scenario-s2".to_string(),
        }),
    ) {
        DaemonResponse::RegisterSession(r) => r.session_id,
        other => panic!("unexpected response: {other:?}"),
    };

    let task_id = match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_id),
            goal: "summarize memory and runtime".to_string(),
            idempotency_key: None,
        }),
    ) {
        DaemonResponse::SubmitTask(r) => {
            assert_eq!(r.task_state, "succeeded");
            r.task_id
        }
        other => panic!("unexpected response: {other:?}"),
    };

    match send_request(
        &socket,
        &DaemonRequest::GetTrace(GetTraceRequest {
            task_id: task_id.clone(),
        }),
    ) {
        DaemonResponse::GetTrace(r) => {
            assert!(r.trace.events.iter().any(|e| e.event_kind == "fit_loop_adjusted"));
            assert!(r.trace.events.iter().any(|e| e.event_kind == "fit_loop_fitted"));
            assert!(r.trace.events.iter().any(|e| e.event_kind == "model_output_received"));
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
            assert!(r.artifacts.iter().any(|a| {
                a.artifact_kind == "fit_loop_decision"
                    && a.summary.contains("final_decision=fitted")
            }));
            assert!(r.artifacts.iter().any(|a| a.artifact_kind == "final_result"));
        }
        other => panic!("unexpected response: {other:?}"),
    }

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
    let _ = fs::remove_file(&config);
}

#[test]
fn scenario_s4_non_convergent_fit_loop_fails_without_success_records() {
    let socket = unique_path("sharo-scenario-s4", ".sock");
    let store = unique_path("sharo-scenario-s4", ".json");
    let config = write_reasoning_failure_config("sharo-scenario-s4");

    let mut daemon = Command::new(daemon_bin())
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

    let session_id = match send_request(
        &socket,
        &DaemonRequest::RegisterSession(RegisterSessionRequest {
            session_label: "scenario-s4".to_string(),
        }),
    ) {
        DaemonResponse::RegisterSession(r) => r.session_id,
        other => panic!("unexpected response: {other:?}"),
    };

    let task_id = match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_id),
            goal: "this goal is intentionally too long".to_string(),
            idempotency_key: Some("idem-s4".to_string()),
        }),
    ) {
        DaemonResponse::SubmitTask(r) => {
            assert_eq!(r.task_state, "failed");
            assert!(r.summary.contains("context_policy_fit_failed"));
            r.task_id
        }
        other => panic!("unexpected response: {other:?}"),
    };

    match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some("session-000001".to_string()),
            goal: "this goal is intentionally too long".to_string(),
            idempotency_key: Some("idem-s4".to_string()),
        }),
    ) {
        DaemonResponse::SubmitTask(r) => {
            assert_eq!(r.task_id, task_id);
            assert_eq!(r.task_state, "failed");
        }
        other => panic!("unexpected response: {other:?}"),
    };

    match send_request(
        &socket,
        &DaemonRequest::GetTask(GetTaskRequest {
            task_id: task_id.clone(),
        }),
    ) {
        DaemonResponse::GetTask(r) => {
            assert_eq!(r.task.task_state, "failed");
            assert!(
                r.task
                    .blocking_reason
                    .as_deref()
                    .unwrap_or("")
                    .contains("context_policy_fit_failed")
            );
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
            assert!(r.trace.events.iter().any(|e| e.event_kind == "fit_loop_adjusted"));
            assert!(r.trace.events.iter().any(|e| e.event_kind == "fit_loop_failed"));
            assert!(!r.trace.events.iter().any(|e| e.event_kind == "model_output_received"));
            assert_trace_monotonic(&r.trace);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    match send_request(
        &socket,
        &DaemonRequest::GetArtifacts(GetArtifactsRequest { task_id }),
    ) {
        DaemonResponse::GetArtifacts(r) => {
            let kinds: Vec<&str> = r.artifacts.iter().map(|a| a.artifact_kind.as_str()).collect();
            assert!(kinds.contains(&"fit_loop_decision"));
            assert!(kinds.contains(&"failure_record"));
            assert!(!kinds.contains(&"model_output"));
            assert!(!kinds.contains(&"final_result"));
            assert!(r.artifacts.iter().any(|a| {
                a.artifact_kind == "fit_loop_decision"
                    && a.summary.contains("iterations=")
                    && !a.summary.contains("iterations=0")
            }));
        }
        other => panic!("unexpected response: {other:?}"),
    }

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
    let _ = fs::remove_file(&config);
}

#[test]
fn scenario_s3_provider_auth_failure_returns_error_without_persisted_task() {
    let socket = unique_path("sharo-scenario-s3", ".sock");
    let store = unique_path("sharo-scenario-s3", ".json");
    let config = write_openai_missing_auth_config("sharo-scenario-s3");

    let mut daemon = Command::new(daemon_bin())
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

    let session_id = match send_request(
        &socket,
        &DaemonRequest::RegisterSession(RegisterSessionRequest {
            session_label: "scenario-s3".to_string(),
        }),
    ) {
        DaemonResponse::RegisterSession(r) => r.session_id,
        other => panic!("unexpected response: {other:?}"),
    };

    let first_error = match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_id),
            goal: "read one context item".to_string(),
            idempotency_key: Some("idem-s3".to_string()),
        }),
    ) {
        DaemonResponse::Error { message } => {
            assert!(message.contains("missing auth env var SHARO_TEST_MISSING_OPENAI_KEY"));
            message
        }
        other => panic!("unexpected response: {other:?}"),
    };

    let replayed_error = match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some("session-000001".to_string()),
            goal: "read one context item".to_string(),
            idempotency_key: Some("idem-s3".to_string()),
        }),
    ) {
        DaemonResponse::Error { message } => message,
        other => panic!("unexpected response: {other:?}"),
    };
    assert_eq!(replayed_error, first_error);

    match send_request(
        &socket,
        &DaemonRequest::GetTask(GetTaskRequest {
            task_id: "task-000001".to_string(),
        }),
    ) {
        DaemonResponse::Error { message } => assert!(message.contains("task_not_found")),
        other => panic!("unexpected response: {other:?}"),
    }

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
    let _ = fs::remove_file(&config);
}

#[test]
fn scenario_s5_provider_unavailable_returns_error_without_persisted_task() {
    let socket = unique_path("sharo-scenario-s5", ".sock");
    let store = unique_path("sharo-scenario-s5", ".json");
    let (base_url, server_thread) = start_status_response_server("503 Service Unavailable");
    let config = write_slow_openai_config("sharo-scenario-s5", &base_url);

    let mut daemon = Command::new(daemon_bin())
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

    let session_id = match send_request(
        &socket,
        &DaemonRequest::RegisterSession(RegisterSessionRequest {
            session_label: "scenario-s5".to_string(),
        }),
    ) {
        DaemonResponse::RegisterSession(r) => r.session_id,
        other => panic!("unexpected response: {other:?}"),
    };

    let first_error = match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_id),
            goal: "retryable provider failure".to_string(),
            idempotency_key: Some("idem-s5".to_string()),
        }),
    ) {
        DaemonResponse::Error { message } => {
            assert!(message.contains("provider unavailable status=503"));
            message
        }
        other => panic!("unexpected response: {other:?}"),
    };

    let replayed_error = match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some("session-000001".to_string()),
            goal: "retryable provider failure".to_string(),
            idempotency_key: Some("idem-s5".to_string()),
        }),
    ) {
        DaemonResponse::Error { message } => message,
        other => panic!("unexpected response: {other:?}"),
    };
    assert_eq!(replayed_error, first_error);

    match send_request(
        &socket,
        &DaemonRequest::GetTask(GetTaskRequest {
            task_id: "task-000001".to_string(),
        }),
    ) {
        DaemonResponse::Error { message } => assert!(message.contains("task_not_found")),
        other => panic!("unexpected response: {other:?}"),
    }

    server_thread.join().expect("join status server");
    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
    let _ = fs::remove_file(&config);
}

#[test]
fn scenario_c_overlap_visibility_survives_restart() {
    let socket = unique_path("sharo-scenario-c", ".sock");
    let store = unique_path("sharo-scenario-c", ".json");
    let config = write_deterministic_config("sharo-scenario-c");

    let mut daemon = Command::new(daemon_bin())
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
            "--config-path",
            config.to_str().expect("config path"),
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
            assert_eq!(r.trace.session_id, "session-000002");
            assert!(r.trace.events.iter().any(|e| e.event_kind == "conflict_detected"));
            assert_trace_monotonic(&r.trace);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
    let _ = fs::remove_file(&config);
}

#[test]
fn invalid_manifest_is_blocked_with_explicit_reason() {
    let socket = unique_path("sharo-scenario-manifest", ".sock");
    let store = unique_path("sharo-scenario-manifest", ".json");
    let config = write_deterministic_config("sharo-scenario-manifest");

    let mut daemon = Command::new(daemon_bin())
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
            for artifact in &r.artifacts {
                assert!(!artifact.produced_by_step_id.is_empty());
                assert!(artifact.produced_by_trace_event_sequence > 0);
            }
        }
        other => panic!("unexpected response: {other:?}"),
    }

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
    let _ = fs::remove_file(&config);
}

#[test]
fn approval_list_remains_responsive_during_slow_submit() {
    let socket = unique_path("sharo-scenario-concurrency", ".sock");
    let store = unique_path("sharo-scenario-concurrency", ".json");
    let (base_url, server_thread) = start_delayed_response_server(Duration::from_millis(500));
    let config = write_slow_openai_config("sharo-scenario-concurrency", &base_url);

    let mut daemon = Command::new(daemon_bin())
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

    let submit_stream = {
        let mut connected = None;
        for _ in 0..80 {
            match UnixStream::connect(&socket) {
                Ok(stream) => {
                    connected = Some(stream);
                    break;
                }
                Err(_) => thread::sleep(Duration::from_millis(15)),
            }
        }
        connected.expect("connect slow submit stream")
    };
    let submit_thread = thread::spawn(move || {
        send_request_on_stream(
            submit_stream,
            &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
                session_id: Some("session-concurrency".to_string()),
                goal: "slow approval-list submit".to_string(),
                idempotency_key: None,
            }),
        )
    });

    thread::sleep(Duration::from_millis(75));

    let approval_start = SystemTime::now();
    let approvals_response = send_request(&socket, &DaemonRequest::ListPendingApprovals);
    let approval_elapsed = approval_start.elapsed().expect("approval elapsed");

    match approvals_response {
        DaemonResponse::ListPendingApprovals(r) => assert!(r.approvals.is_empty()),
        other => panic!("unexpected response: {other:?}"),
    }
    assert!(
        approval_elapsed < Duration::from_millis(450),
        "approval list request took {:?} while slow submit was running",
        approval_elapsed
    );

    match submit_thread.join().expect("submit thread join") {
        DaemonResponse::SubmitTask(r) => assert_eq!(r.task_state, "succeeded"),
        other => panic!("unexpected response: {other:?}"),
    }

    server_thread.join().expect("delayed server join");
    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
    let _ = fs::remove_file(&config);
}

#[test]
fn store_file_permissions_are_restricted() {
    let socket = unique_path("sharo-store-perms", ".sock");
    let store = unique_path("sharo-store-perms", ".json");
    let config = write_deterministic_config("sharo-store-perms");

    let mut daemon = Command::new(daemon_bin())
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
            "--config-path",
            config.to_str().expect("config path"),
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
    let _ = fs::remove_file(&config);
}
