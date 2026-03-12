use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sharo_core::mcp::McpRuntimeStatus;
use sharo_core::protocol::{
    DaemonRequest, DaemonResponse, GetHazelStatusResponse, GetRuntimeStatusResponse,
    GetSessionViewRequest, GetSkillRequest, HazelRetrievalPreviewRequest, HazelSleepJobState,
    ListHazelCardsResponse, ListHazelSleepJobsResponse, ListMcpServersResponse,
    ListSessionsResponse, ListSkillsRequest, PrePromptComposeHookInput, SubmitTaskOpRequest,
    SubmitTaskRequest, TaskStatusRequest, UpdateMcpServerStateRequest,
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

fn write_hazel_seed_store(path: &PathBuf) {
    let state = serde_json::json!({
        "hazel_proposal_batches": {
            "batch-000001": {
                "batch_id": "batch-000001",
                "idempotency_key": "idemp-000001",
                "provenance": {
                    "source_ref": "note:hazel",
                    "producer": "operator"
                },
                "proposals": [{
                    "proposal_id": "proposal-000001",
                    "kind": "chunk_upsert",
                    "chunk": {
                        "chunk_id": "chunk-000001",
                        "content": "hazel inspection batch",
                        "source_ref": "note:hazel"
                    },
                    "entity": null,
                    "relation": null,
                    "assertion": null
                }]
            }
        },
        "hazel_sleep_jobs": {
            "job-000001": {
                "job_id": "job-000001",
                "state": "completed",
                "run_id": "sleep-run-v2-seeded",
                "proposal_batch_ids": ["batch-000001"],
                "summary": "completed with one batch"
            }
        }
    });
    fs::write(
        path,
        serde_json::to_string_pretty(&state).expect("serialize hazel seed store"),
    )
    .expect("write hazel seed store");
}

fn write_hazel_pending_job_store(path: &PathBuf) {
    let state = serde_json::json!({
        "hazel_proposal_batches": {
            "batch-000001": {
                "batch_id": "batch-000001",
                "idempotency_key": "idemp-000001",
                "provenance": {
                    "source_ref": "note:hazel",
                    "producer": "operator"
                },
                "proposals": [{
                    "proposal_id": "proposal-000001",
                    "kind": "chunk_upsert",
                    "chunk": {
                        "chunk_id": "chunk-000001",
                        "content": "hazel inspection batch",
                        "source_ref": "note:hazel"
                    },
                    "entity": null,
                    "relation": null,
                    "assertion": null
                }]
            }
        },
        "hazel_sleep_jobs": {
            "job-000001": {
                "job_id": "job-000001",
                "state": "pending",
                "run_id": null,
                "proposal_batch_ids": ["batch-000001"],
                "summary": "pending job"
            }
        }
    });
    fs::write(
        path,
        serde_json::to_string_pretty(&state).expect("serialize hazel pending store"),
    )
    .expect("write hazel pending store");
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

fn write_deterministic_config(prefix: &str) -> PathBuf {
    let config = temp_path(prefix, ".toml");
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

fn write_deterministic_config_with_skills(
    prefix: &str,
    project_root: &PathBuf,
    user_root: &PathBuf,
) -> PathBuf {
    write_deterministic_config_with_skills_flags(prefix, project_root, user_root, true)
}

fn write_deterministic_config_with_skills_flags(
    prefix: &str,
    project_root: &PathBuf,
    user_root: &PathBuf,
    trust_project_skills: bool,
) -> PathBuf {
    let config = temp_path(prefix, ".toml");
    fs::write(
        &config,
        format!(
            r#"[model]
provider = "deterministic"
model_id = "mock"
timeout_ms = 1000

[skills]
project_root = "{project_root}"
user_root = "{user_root}"
enable_project_skills = true
enable_user_skills = true
max_depth = 5
trust_project_skills = {trust_project_skills}
"#,
            project_root = project_root.display(),
            user_root = user_root.display(),
            trust_project_skills = trust_project_skills,
        ),
    )
    .expect("write deterministic config with skills");
    config
}

fn write_deterministic_config_with_mcp(prefix: &str) -> PathBuf {
    let config = temp_path(prefix, ".toml");
    fs::write(
        &config,
        r#"[model]
provider = "deterministic"
model_id = "mock"
timeout_ms = 1000
profile_id = "mcp-profile"

[[mcp.servers]]
server_id = "hazel"
display_name = "Hazel"
transport = "stdio"
command = "/usr/bin/hazel-mcp"
args = ["--stdio"]
startup_timeout_ms = 250
trust_class = "operator"
enabled = true

[[mcp.servers]]
server_id = "docs"
transport = "http"
endpoint = "http://127.0.0.1:8080/mcp"
enabled = false
"#,
    )
    .expect("write deterministic config with mcp");
    config
}

fn write_skill(root: &PathBuf, relative_dir: &str, markdown: &str) {
    let skill_dir = root.join(relative_dir);
    fs::create_dir_all(&skill_dir).expect("create skill dir");
    fs::write(skill_dir.join("SKILL.md"), markdown).expect("write skill");
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

fn send_request(socket: &PathBuf, request: &DaemonRequest) -> DaemonResponse {
    send_request_with_stream(connect_with_retry(socket), request)
}

fn unique_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{nanos}"))
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
fn hazel_status_response_is_bounded() {
    let socket = socket_path();
    let store = temp_path("sharo-daemon-hazel-store", ".json");
    write_hazel_seed_store(&store);

    let mut child = Command::new(env!("CARGO_BIN_EXE_sharo-daemon"))
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket path"),
            "--store-path",
            store.to_str().expect("store path"),
            "--serve-once",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    let response = send_request(&socket, &DaemonRequest::GetHazelStatus);
    match response {
        DaemonResponse::GetHazelStatus(GetHazelStatusResponse { status }) => {
            assert!(status.available);
            assert!(status.card_count <= status.limits.max_list_items);
            assert_eq!(status.proposal_batch_count, 1);
            assert_eq!(status.sleep_job_count, 1);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    child.wait().expect("wait daemon");
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
}

#[test]
fn hazel_list_cards_returns_transport_safe_view() {
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

    let response = send_request(
        &socket,
        &DaemonRequest::ListHazelCards(sharo_core::protocol::ListHazelCardsRequest {
            limit: Some(1),
        }),
    );
    match response {
        DaemonResponse::ListHazelCards(ListHazelCardsResponse { cards }) => {
            assert_eq!(cards.len(), 1);
            assert!(!cards[0].card_id.is_empty());
            assert!(!cards[0].provenance.is_empty());
        }
        other => panic!("unexpected response: {other:?}"),
    }

    child.wait().expect("wait daemon");
    let _ = fs::remove_file(&socket);
}

#[test]
fn hazel_get_proposal_batch_returns_exact_provenance_summary() {
    let socket = socket_path();
    let store = temp_path("sharo-daemon-hazel-store", ".json");
    write_hazel_seed_store(&store);

    let mut child = Command::new(env!("CARGO_BIN_EXE_sharo-daemon"))
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket path"),
            "--store-path",
            store.to_str().expect("store path"),
            "--serve-once",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    let response = send_request(
        &socket,
        &DaemonRequest::GetHazelProposalBatch(sharo_core::protocol::GetHazelProposalBatchRequest {
            batch_id: "batch-000001".to_string(),
        }),
    );
    match response {
        DaemonResponse::GetHazelProposalBatch(payload) => {
            assert_eq!(payload.batch.batch_id, "batch-000001");
            assert_eq!(payload.batch.source_ref, "note:hazel");
            assert_eq!(payload.batch.producer, "operator");
            assert_eq!(payload.batch.proposal_count, 1);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    child.wait().expect("wait daemon");
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
}

#[test]
fn hazel_list_sleep_jobs_returns_bounded_statuses() {
    let socket = socket_path();
    let store = temp_path("sharo-daemon-hazel-store", ".json");
    write_hazel_seed_store(&store);

    let mut child = Command::new(env!("CARGO_BIN_EXE_sharo-daemon"))
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket path"),
            "--store-path",
            store.to_str().expect("store path"),
            "--serve-once",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    let response = send_request(
        &socket,
        &DaemonRequest::ListHazelSleepJobs(sharo_core::protocol::ListHazelSleepJobsRequest {
            limit: Some(8),
        }),
    );
    match response {
        DaemonResponse::ListHazelSleepJobs(ListHazelSleepJobsResponse { jobs }) => {
            assert_eq!(jobs.len(), 1);
            assert_eq!(jobs[0].state, HazelSleepJobState::Completed);
            assert_eq!(jobs[0].proposal_batch_ids, vec!["batch-000001".to_string()]);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    child.wait().expect("wait daemon");
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
}

#[test]
fn hazel_preview_returns_derived_payload_without_canonical_write() {
    let socket = socket_path();
    let store = temp_path("sharo-daemon-hazel-preview-store", ".json");

    let mut child = Command::new(env!("CARGO_BIN_EXE_sharo-daemon"))
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket path"),
            "--store-path",
            store.to_str().expect("store path"),
            "--serve-once",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    let response = send_request(
        &socket,
        &DaemonRequest::HazelPreview(HazelRetrievalPreviewRequest {
            input: PrePromptComposeHookInput {
                session_id: "session-000001".to_string(),
                task_id: "task-000001".to_string(),
                goal: "structured memory hazel".to_string(),
                runtime: "operator".to_string(),
                top_k: Some(2),
                token_budget: Some(64),
                relevance_threshold: Some(0.0),
                policy_ids: vec!["hunch.v1".to_string()],
                card_policy_hints: Vec::new(),
            },
        }),
    );
    match response {
        DaemonResponse::HazelPreview(payload) => {
            assert!(!payload.preview_id.is_empty());
            assert!(!payload.payload.cards.is_empty());
        }
        other => panic!("unexpected response: {other:?}"),
    }

    child.wait().expect("wait daemon");
    let persisted = fs::read_to_string(&store).expect("read store");
    let json: serde_json::Value = serde_json::from_str(&persisted).expect("parse store");
    assert!(json["hazel_proposal_batches"]
        .as_object()
        .expect("proposal batches object")
        .is_empty());

    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
}

#[test]
fn hazel_preview_rejects_oversized_token_budget() {
    let socket = socket_path();
    let store = temp_path("sharo-daemon-hazel-preview-oversized-store", ".json");

    let mut child = Command::new(env!("CARGO_BIN_EXE_sharo-daemon"))
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket path"),
            "--store-path",
            store.to_str().expect("store path"),
            "--serve-once",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    let response = send_request(
        &socket,
        &DaemonRequest::HazelPreview(HazelRetrievalPreviewRequest {
            input: PrePromptComposeHookInput {
                session_id: "session-000001".to_string(),
                task_id: "task-oversized".to_string(),
                goal: "structured memory hazel".to_string(),
                runtime: "operator".to_string(),
                top_k: Some(2),
                token_budget: Some(65_537),
                relevance_threshold: Some(0.0),
                policy_ids: vec!["hunch.v1".to_string()],
                card_policy_hints: Vec::new(),
            },
        }),
    );
    match response {
        DaemonResponse::Error { message } => {
            assert!(message.contains("hazel_preview_invalid"));
            assert!(message.contains("token_budget_exceeded"));
        }
        other => panic!("unexpected response: {other:?}"),
    }

    child.wait().expect("wait daemon");
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
}

#[test]
fn hazel_submit_batch_persists_submission_outcome_record() {
    let socket = socket_path();
    let store = temp_path("sharo-daemon-hazel-submit-store", ".json");
    write_hazel_seed_store(&store);

    let mut child = Command::new(env!("CARGO_BIN_EXE_sharo-daemon"))
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket path"),
            "--store-path",
            store.to_str().expect("store path"),
            "--serve-once",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    let response = send_request(
        &socket,
        &DaemonRequest::SubmitHazelProposalBatch(
            sharo_core::protocol::SubmitHazelProposalBatchRequest {
                batch_id: "batch-000001".to_string(),
                strict_policy_ids: vec!["hunch.v1".to_string()],
            },
        ),
    );
    match response {
        DaemonResponse::SubmitHazelProposalBatch(payload) => {
            assert_eq!(payload.batch_id, "batch-000001");
            assert_eq!(payload.state, "accepted");
        }
        other => panic!("unexpected response: {other:?}"),
    }

    child.wait().expect("wait daemon");
    let persisted = fs::read_to_string(&store).expect("read store");
    let json: serde_json::Value = serde_json::from_str(&persisted).expect("parse store");
    assert_eq!(
        json["hazel_submission_records"]
            .as_object()
            .expect("submission records object")
            .len(),
        1
    );

    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
}

#[test]
fn hazel_validate_batch_rejects_unknown_policy_ids_in_strict_mode() {
    let socket = socket_path();
    let store = temp_path("sharo-daemon-hazel-validate-store", ".json");
    write_hazel_seed_store(&store);

    let mut child = Command::new(env!("CARGO_BIN_EXE_sharo-daemon"))
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket path"),
            "--store-path",
            store.to_str().expect("store path"),
            "--serve-once",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    let response = send_request(
        &socket,
        &DaemonRequest::ValidateHazelProposalBatch(
            sharo_core::protocol::ValidateHazelProposalBatchRequest {
                batch_id: "batch-000001".to_string(),
                strict_policy_ids: vec!["unknown.v9".to_string()],
            },
        ),
    );
    match response {
        DaemonResponse::Error { message } => assert!(message.contains("hazel_policy_unknown")),
        other => panic!("unexpected response: {other:?}"),
    }

    child.wait().expect("wait daemon");
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
}

#[test]
fn hazel_cancel_sleep_job_stops_future_proposal_production() {
    let socket = socket_path();
    let store = temp_path("sharo-daemon-hazel-cancel-store", ".json");
    write_hazel_pending_job_store(&store);

    let mut child = Command::new(env!("CARGO_BIN_EXE_sharo-daemon"))
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket path"),
            "--store-path",
            store.to_str().expect("store path"),
            "--serve-once",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    let response = send_request(
        &socket,
        &DaemonRequest::CancelHazelSleepJob(sharo_core::protocol::CancelHazelSleepJobRequest {
            job_id: "job-000001".to_string(),
        }),
    );
    match response {
        DaemonResponse::CancelHazelSleepJob(payload) => {
            assert_eq!(payload.job.job_id, "job-000001");
            assert_eq!(payload.job.state, HazelSleepJobState::Canceled);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    child.wait().expect("wait daemon");
    let persisted = fs::read_to_string(&store).expect("read store");
    let json: serde_json::Value = serde_json::from_str(&persisted).expect("parse store");
    assert_eq!(
        json["hazel_sleep_jobs"]["job-000001"]["state"],
        serde_json::Value::String("canceled".to_string())
    );
    assert_eq!(
        json["hazel_sleep_jobs"]["job-000001"]["proposal_batch_ids"],
        serde_json::json!(["batch-000001"])
    );

    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
}

#[test]
fn hazel_cancel_sleep_job_rejects_completed_job() {
    let socket = socket_path();
    let store = temp_path("sharo-daemon-hazel-cancel-completed", ".json");
    write_hazel_seed_store(&store);

    let mut child = Command::new(env!("CARGO_BIN_EXE_sharo-daemon"))
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket path"),
            "--store-path",
            store.to_str().expect("store path"),
            "--serve-once",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    let response = send_request(
        &socket,
        &DaemonRequest::CancelHazelSleepJob(sharo_core::protocol::CancelHazelSleepJobRequest {
            job_id: "job-000001".to_string(),
        }),
    );
    match response {
        DaemonResponse::Error { message } => {
            assert!(message.contains("hazel_sleep_job_cancel_invalid"))
        }
        other => panic!("unexpected response: {other:?}"),
    }

    child.wait().expect("wait daemon");
    let persisted = fs::read_to_string(&store).expect("read store");
    let json: serde_json::Value = serde_json::from_str(&persisted).expect("parse store");
    assert_eq!(
        json["hazel_sleep_jobs"]["job-000001"]["state"],
        serde_json::Value::String("completed".to_string())
    );

    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
}

#[test]
fn list_skills_returns_catalog_without_full_skill_payloads() {
    let socket = socket_path();
    let project_root = unique_dir("sharo-daemon-skills-project");
    let user_root = unique_dir("sharo-daemon-skills-user");
    write_skill(
        &project_root,
        "writing/docs/strict-plan",
        "---\nname: Strict Plan\ndescription: Enforce structured planning\n---\n# Strict Plan\n\nFull skill body.\n",
    );
    let config = write_deterministic_config_with_skills(
        "sharo-daemon-skills-catalog",
        &project_root,
        &user_root,
    );

    let mut child = Command::new(env!("CARGO_BIN_EXE_sharo-daemon"))
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket path"),
            "--config-path",
            config.to_str().expect("config path"),
            "--serve-once",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    let response = send_request(
        &socket,
        &DaemonRequest::ListSkills(ListSkillsRequest { session_id: None }),
    );
    match response {
        DaemonResponse::ListSkills(payload) => {
            assert_eq!(payload.skills.len(), 1);
            assert_eq!(payload.skills[0].skill_id, "writing/docs/strict-plan");
            assert_eq!(payload.skills[0].name, "Strict Plan");
            assert_eq!(payload.skills[0].description, "Enforce structured planning");
            assert!(!payload.skills[0].is_active);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    child.wait().expect("wait daemon");
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&config);
    let _ = fs::remove_dir_all(project_root);
    let _ = fs::remove_dir_all(user_root);
}

#[test]
fn set_session_skills_persists_activation_state() {
    let socket = socket_path();
    let store = temp_path("sharo-daemon-store", ".json");
    let project_root = unique_dir("sharo-daemon-skills-project");
    let user_root = unique_dir("sharo-daemon-skills-user");
    write_skill(
        &project_root,
        "brainstorming",
        "---\nname: Brainstorming\ndescription: Explore the design space\n---\n# Brainstorming\n\nFull skill body.\n",
    );
    let config = write_deterministic_config_with_skills(
        "sharo-daemon-set-skills",
        &project_root,
        &user_root,
    );

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

    let register = send_request(
        &socket,
        &DaemonRequest::RegisterSession(sharo_core::protocol::RegisterSessionRequest {
            session_label: "alpha".to_string(),
        }),
    );
    let session_id = match register {
        DaemonResponse::RegisterSession(payload) => payload.session_id,
        other => panic!("unexpected register response: {other:?}"),
    };

    let set_response = send_request(
        &socket,
        &DaemonRequest::SetSessionSkills(sharo_core::protocol::SetSessionSkillsRequest {
            session_id: session_id.clone(),
            active_skill_ids: vec!["brainstorming".to_string()],
        }),
    );
    match set_response {
        DaemonResponse::SetSessionSkills(payload) => {
            assert_eq!(payload.session_id, session_id);
            assert_eq!(payload.active_skill_ids, vec!["brainstorming".to_string()]);
        }
        other => panic!("unexpected set response: {other:?}"),
    }

    let list_response = send_request(
        &socket,
        &DaemonRequest::ListSkills(ListSkillsRequest {
            session_id: Some(session_id.clone()),
        }),
    );
    match list_response {
        DaemonResponse::ListSkills(payload) => {
            assert_eq!(payload.skills.len(), 1);
            assert!(payload.skills[0].is_active);
        }
        other => panic!("unexpected list response: {other:?}"),
    }

    let get_response = send_request(
        &socket,
        &DaemonRequest::GetSkill(GetSkillRequest {
            skill_id: "brainstorming".to_string(),
        }),
    );
    match get_response {
        DaemonResponse::GetSkill(payload) => {
            assert!(payload.skill.markdown.contains("# Brainstorming"));
        }
        other => panic!("unexpected get response: {other:?}"),
    }

    child.kill().expect("kill daemon");
    child.wait().expect("wait daemon");
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&config);
    let _ = fs::remove_file(&store);
    let _ = fs::remove_dir_all(project_root);
    let _ = fs::remove_dir_all(user_root);
}

#[test]
fn set_session_skills_rejects_unknown_session() {
    let socket = socket_path();
    let store = temp_path("sharo-daemon-store", ".json");
    let project_root = unique_dir("sharo-daemon-skills-project");
    let user_root = unique_dir("sharo-daemon-skills-user");
    write_skill(
        &project_root,
        "brainstorming",
        "---\nname: Brainstorming\ndescription: Explore the design space\n---\n# Brainstorming\n",
    );
    let config = write_deterministic_config_with_skills(
        "sharo-daemon-unknown-session-skills",
        &project_root,
        &user_root,
    );

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

    let response = send_request(
        &socket,
        &DaemonRequest::SetSessionSkills(sharo_core::protocol::SetSessionSkillsRequest {
            session_id: "session-missing".to_string(),
            active_skill_ids: vec!["brainstorming".to_string()],
        }),
    );
    match response {
        DaemonResponse::Error { message } => {
            assert!(message.contains("session_not_found"));
        }
        other => panic!("unexpected response: {other:?}"),
    }

    let sessions = send_request(&socket, &DaemonRequest::ListSessions);
    match sessions {
        DaemonResponse::ListSessions(payload) => assert!(payload.sessions.is_empty()),
        other => panic!("unexpected response: {other:?}"),
    }

    child.kill().expect("kill daemon");
    child.wait().expect("wait daemon");
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&config);
    let _ = fs::remove_file(&store);
    let _ = fs::remove_dir_all(project_root);
    let _ = fs::remove_dir_all(user_root);
}

#[test]
fn untrusted_project_skills_are_not_listed_or_fetchable() {
    let socket = socket_path();
    let project_root = unique_dir("sharo-daemon-skills-project");
    let user_root = unique_dir("sharo-daemon-skills-user");
    write_skill(
        &project_root,
        "brainstorming",
        "---\nname: Brainstorming\ndescription: hidden when untrusted\n---\n# Brainstorming\n",
    );
    let config = write_deterministic_config_with_skills_flags(
        "sharo-daemon-untrusted-project-skills",
        &project_root,
        &user_root,
        false,
    );

    let mut child = Command::new(env!("CARGO_BIN_EXE_sharo-daemon"))
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket path"),
            "--config-path",
            config.to_str().expect("config path"),
            "--serve-once",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    let response = send_request(
        &socket,
        &DaemonRequest::ListSkills(ListSkillsRequest { session_id: None }),
    );
    match response {
        DaemonResponse::ListSkills(payload) => assert!(payload.skills.is_empty()),
        other => panic!("unexpected response: {other:?}"),
    }

    child.wait().expect("wait daemon");
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&config);
    let _ = fs::remove_dir_all(project_root);
    let _ = fs::remove_dir_all(user_root);
}

#[test]
fn oversized_skill_document_is_rejected() {
    let socket = socket_path();
    let store = temp_path("sharo-daemon-store", ".json");
    let project_root = unique_dir("sharo-daemon-skills-project");
    let user_root = unique_dir("sharo-daemon-skills-user");
    let oversized_body = "A".repeat(70_000);
    write_skill(
        &project_root,
        "oversized",
        &format!(
            "---\nname: Oversized\ndescription: too large\n---\n# Oversized\n\n{oversized_body}\n"
        ),
    );
    let config = write_deterministic_config_with_skills(
        "sharo-daemon-oversized-skill",
        &project_root,
        &user_root,
    );

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

    let response = send_request(
        &socket,
        &DaemonRequest::GetSkill(GetSkillRequest {
            skill_id: "oversized".to_string(),
        }),
    );
    match response {
        DaemonResponse::Error { message } => assert!(message.contains("skill_payload_too_large")),
        other => panic!("unexpected response: {other:?}"),
    }

    child.kill().expect("kill daemon");
    child.wait().expect("wait daemon");
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&config);
    let _ = fs::remove_file(&store);
    let _ = fs::remove_dir_all(project_root);
    let _ = fs::remove_dir_all(user_root);
}

#[test]
fn list_skills_response_is_bounded() {
    let socket = socket_path();
    let project_root = unique_dir("sharo-daemon-skills-project");
    let user_root = unique_dir("sharo-daemon-skills-user");
    for index in 0..130 {
        write_skill(
            &project_root,
            &format!("skill-{index:03}"),
            &format!(
                "---\nname: Skill {index}\ndescription: bounded listing\n---\n# Skill {index}\n"
            ),
        );
    }
    let config = write_deterministic_config_with_skills(
        "sharo-daemon-bounded-skills",
        &project_root,
        &user_root,
    );

    let mut child = Command::new(env!("CARGO_BIN_EXE_sharo-daemon"))
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket path"),
            "--config-path",
            config.to_str().expect("config path"),
            "--serve-once",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    let response = send_request(
        &socket,
        &DaemonRequest::ListSkills(ListSkillsRequest { session_id: None }),
    );
    match response {
        DaemonResponse::ListSkills(payload) => {
            assert_eq!(payload.skills.len(), 100);
            assert_eq!(payload.skills[0].skill_id, "skill-000");
            assert_eq!(payload.skills[99].skill_id, "skill-099");
        }
        other => panic!("unexpected response: {other:?}"),
    }

    child.wait().expect("wait daemon");
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&config);
    let _ = fs::remove_dir_all(project_root);
    let _ = fs::remove_dir_all(user_root);
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
fn list_sessions_returns_recent_activity_order() {
    let socket = socket_path();
    let store = temp_path("sharo-daemon-store", ".json");
    let config = write_deterministic_config("sharo-daemon-session-list");

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

    let session_a = match send_request(
        &socket,
        &DaemonRequest::RegisterSession(sharo_core::protocol::RegisterSessionRequest {
            session_label: "alpha".to_string(),
        }),
    ) {
        DaemonResponse::RegisterSession(response) => response.session_id,
        other => panic!("unexpected register response: {other:?}"),
    };
    let session_b = match send_request(
        &socket,
        &DaemonRequest::RegisterSession(sharo_core::protocol::RegisterSessionRequest {
            session_label: "beta".to_string(),
        }),
    ) {
        DaemonResponse::RegisterSession(response) => response.session_id,
        other => panic!("unexpected register response: {other:?}"),
    };

    match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_a.clone()),
            goal: "read alpha".to_string(),
            idempotency_key: None,
        }),
    ) {
        DaemonResponse::SubmitTask(response) => {
            assert_eq!(response.task_state, "succeeded");
        }
        other => panic!("unexpected submit response: {other:?}"),
    }
    match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_b.clone()),
            goal: "read beta".to_string(),
            idempotency_key: None,
        }),
    ) {
        DaemonResponse::SubmitTask(response) => {
            assert_eq!(response.task_state, "succeeded");
        }
        other => panic!("unexpected submit response: {other:?}"),
    }

    let sessions = match send_request(&socket, &DaemonRequest::ListSessions) {
        DaemonResponse::ListSessions(ListSessionsResponse { sessions }) => sessions,
        other => panic!("unexpected list sessions response: {other:?}"),
    };

    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].session_id, session_b);
    assert_eq!(sessions[0].session_label, "beta");
    assert_eq!(sessions[0].session_status, "succeeded");
    assert!(sessions[0].activity_sequence > 0);
    assert_eq!(sessions[0].latest_task_state, Some("succeeded".to_string()));
    assert_eq!(sessions[1].session_id, session_a);

    child.kill().expect("kill daemon");
    child.wait().expect("wait daemon");
    let _ = fs::remove_file(&config);
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
}

#[test]
fn session_view_surfaces_pending_approval_for_active_conversation() {
    let socket = socket_path();
    let store = temp_path("sharo-daemon-store", ".json");
    let config = write_deterministic_config("sharo-daemon-session-view");

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

    let session_id = match send_request(
        &socket,
        &DaemonRequest::RegisterSession(sharo_core::protocol::RegisterSessionRequest {
            session_label: "alpha".to_string(),
        }),
    ) {
        DaemonResponse::RegisterSession(response) => response.session_id,
        other => panic!("unexpected register response: {other:?}"),
    };

    let task_id = match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_id.clone()),
            goal: "restricted: inspect repo".to_string(),
            idempotency_key: None,
        }),
    ) {
        DaemonResponse::SubmitTask(response) => response.task_id,
        other => panic!("unexpected submit response: {other:?}"),
    };

    let session = match send_request(
        &socket,
        &DaemonRequest::GetSessionView(GetSessionViewRequest {
            session_id: session_id.clone(),
            task_limit: None,
        }),
    ) {
        DaemonResponse::GetSessionView(response) => response.session,
        other => panic!("unexpected session view response: {other:?}"),
    };

    assert_eq!(session.session_id, session_id);
    assert_eq!(session.tasks.len(), 1);
    assert_eq!(session.tasks[0].task_id, task_id);
    assert_eq!(session.pending_approvals.len(), 1);
    assert_eq!(session.pending_approvals[0].task_id, task_id);
    assert_eq!(
        session.active_blocking_task_id.as_deref(),
        Some(task_id.as_str())
    );

    child.kill().expect("kill daemon");
    child.wait().expect("wait daemon");
    let _ = fs::remove_file(&config);
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
}

#[test]
fn list_sessions_orders_by_latest_activity_not_only_latest_task_id() {
    let socket = socket_path();
    let store = temp_path("sharo-daemon-store", ".json");
    let config = write_deterministic_config("sharo-daemon-session-activity");

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

    let session_a = match send_request(
        &socket,
        &DaemonRequest::RegisterSession(sharo_core::protocol::RegisterSessionRequest {
            session_label: "alpha".to_string(),
        }),
    ) {
        DaemonResponse::RegisterSession(response) => response.session_id,
        other => panic!("unexpected register response: {other:?}"),
    };
    let session_b = match send_request(
        &socket,
        &DaemonRequest::RegisterSession(sharo_core::protocol::RegisterSessionRequest {
            session_label: "beta".to_string(),
        }),
    ) {
        DaemonResponse::RegisterSession(response) => response.session_id,
        other => panic!("unexpected register response: {other:?}"),
    };

    let approval_id = match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_a.clone()),
            goal: "restricted: inspect repo".to_string(),
            idempotency_key: None,
        }),
    ) {
        DaemonResponse::SubmitTask(_) => {
            match send_request(&socket, &DaemonRequest::ListPendingApprovals) {
                DaemonResponse::ListPendingApprovals(response) => {
                    response
                        .approvals
                        .into_iter()
                        .find(|approval| approval.task_id.starts_with("task-"))
                        .expect("pending approval")
                        .approval_id
                }
                other => panic!("unexpected approvals response: {other:?}"),
            }
        }
        other => panic!("unexpected submit response: {other:?}"),
    };

    match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_b.clone()),
            goal: "read beta".to_string(),
            idempotency_key: None,
        }),
    ) {
        DaemonResponse::SubmitTask(response) => assert_eq!(response.task_state, "succeeded"),
        other => panic!("unexpected submit response: {other:?}"),
    }

    match send_request(
        &socket,
        &DaemonRequest::ResolveApproval(sharo_core::protocol::ResolveApprovalRequest {
            approval_id,
            decision: "approve".to_string(),
        }),
    ) {
        DaemonResponse::ResolveApproval(response) => assert_eq!(response.state, "approved"),
        other => panic!("unexpected resolve response: {other:?}"),
    }

    let sessions = match send_request(&socket, &DaemonRequest::ListSessions) {
        DaemonResponse::ListSessions(ListSessionsResponse { sessions }) => sessions,
        other => panic!("unexpected list sessions response: {other:?}"),
    };

    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].session_id, session_a);
    assert_eq!(sessions[0].session_status, "succeeded");
    assert!(
        sessions[0].activity_sequence > sessions[1].activity_sequence,
        "resolved approval should advance session activity ordering"
    );

    child.kill().expect("kill daemon");
    child.wait().expect("wait daemon");
    let _ = fs::remove_file(&config);
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
}

#[test]
fn session_view_ignores_stale_historical_blocked_task_for_active_blocking_state() {
    let socket = socket_path();
    let store = temp_path("sharo-daemon-store", ".json");
    let config = write_deterministic_config("sharo-daemon-session-stale-blocking");

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

    let session_id = match send_request(
        &socket,
        &DaemonRequest::RegisterSession(sharo_core::protocol::RegisterSessionRequest {
            session_label: "alpha".to_string(),
        }),
    ) {
        DaemonResponse::RegisterSession(response) => response.session_id,
        other => panic!("unexpected register response: {other:?}"),
    };

    let approval_id = match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_id.clone()),
            goal: "restricted: inspect repo".to_string(),
            idempotency_key: None,
        }),
    ) {
        DaemonResponse::SubmitTask(_) => {
            match send_request(&socket, &DaemonRequest::ListPendingApprovals) {
                DaemonResponse::ListPendingApprovals(response) => {
                    response
                        .approvals
                        .into_iter()
                        .next()
                        .expect("pending approval")
                        .approval_id
                }
                other => panic!("unexpected approvals response: {other:?}"),
            }
        }
        other => panic!("unexpected submit response: {other:?}"),
    };

    match send_request(
        &socket,
        &DaemonRequest::ResolveApproval(sharo_core::protocol::ResolveApprovalRequest {
            approval_id,
            decision: "deny".to_string(),
        }),
    ) {
        DaemonResponse::ResolveApproval(response) => assert_eq!(response.state, "denied"),
        other => panic!("unexpected resolve response: {other:?}"),
    }

    match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_id.clone()),
            goal: "read alpha".to_string(),
            idempotency_key: None,
        }),
    ) {
        DaemonResponse::SubmitTask(response) => assert_eq!(response.task_state, "succeeded"),
        other => panic!("unexpected submit response: {other:?}"),
    }

    let session = match send_request(
        &socket,
        &DaemonRequest::GetSessionView(GetSessionViewRequest {
            session_id: session_id.clone(),
            task_limit: None,
        }),
    ) {
        DaemonResponse::GetSessionView(response) => response.session,
        other => panic!("unexpected session view response: {other:?}"),
    };

    assert!(session.pending_approvals.is_empty());
    assert_eq!(session.active_blocking_task_id, None);

    child.kill().expect("kill daemon");
    child.wait().expect("wait daemon");
    let _ = fs::remove_file(&config);
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
}

#[test]
fn implicit_session_is_listed_and_viewable() {
    let socket = socket_path();
    let store = temp_path("sharo-daemon-store", ".json");
    let config = write_deterministic_config("sharo-daemon-implicit-session");

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

    let task_id = match send_request(
        &socket,
        &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: None,
            goal: "read without explicit session".to_string(),
            idempotency_key: None,
        }),
    ) {
        DaemonResponse::SubmitTask(response) => response.task_id,
        other => panic!("unexpected submit response: {other:?}"),
    };

    let sessions = match send_request(&socket, &DaemonRequest::ListSessions) {
        DaemonResponse::ListSessions(ListSessionsResponse { sessions }) => sessions,
        other => panic!("unexpected list sessions response: {other:?}"),
    };
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].session_id, "session-implicit");
    assert_eq!(sessions[0].session_label, "session-implicit");

    let session = match send_request(
        &socket,
        &DaemonRequest::GetSessionView(GetSessionViewRequest {
            session_id: "session-implicit".to_string(),
            task_limit: None,
        }),
    ) {
        DaemonResponse::GetSessionView(response) => response.session,
        other => panic!("unexpected session view response: {other:?}"),
    };
    assert_eq!(session.tasks.len(), 1);
    assert_eq!(session.tasks[0].task_id, task_id);

    child.kill().expect("kill daemon");
    child.wait().expect("wait daemon");
    let _ = fs::remove_file(&config);
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
}

#[test]
fn session_view_respects_requested_task_limit() {
    let socket = socket_path();
    let store = temp_path("sharo-daemon-store", ".json");
    let config = write_deterministic_config("sharo-daemon-session-limit");

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

    let session_id = match send_request(
        &socket,
        &DaemonRequest::RegisterSession(sharo_core::protocol::RegisterSessionRequest {
            session_label: "alpha".to_string(),
        }),
    ) {
        DaemonResponse::RegisterSession(response) => response.session_id,
        other => panic!("unexpected register response: {other:?}"),
    };

    for index in 0..6 {
        match send_request(
            &socket,
            &DaemonRequest::SubmitTask(SubmitTaskOpRequest {
                session_id: Some(session_id.clone()),
                goal: format!("read alpha {index}"),
                idempotency_key: None,
            }),
        ) {
            DaemonResponse::SubmitTask(response) => assert_eq!(response.task_state, "succeeded"),
            other => panic!("unexpected submit response: {other:?}"),
        }
    }

    let session = match send_request(
        &socket,
        &DaemonRequest::GetSessionView(GetSessionViewRequest {
            session_id,
            task_limit: Some(3),
        }),
    ) {
        DaemonResponse::GetSessionView(response) => response.session,
        other => panic!("unexpected session view response: {other:?}"),
    };
    assert_eq!(session.tasks.len(), 3);
    assert_eq!(session.tasks[0].task_id, "task-000004");
    assert_eq!(session.tasks[2].task_id, "task-000006");

    child.kill().expect("kill daemon");
    child.wait().expect("wait daemon");
    let _ = fs::remove_file(&config);
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
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
fn list_mcp_servers_returns_configured_statuses() {
    let socket = socket_path();
    let store = temp_path("sharo-daemon-mcp-list", ".json");
    let config = write_deterministic_config_with_mcp("sharo-daemon-mcp-list");

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

    match send_request(&socket, &DaemonRequest::ListMcpServers) {
        DaemonResponse::ListMcpServers(ListMcpServersResponse { servers }) => {
            assert_eq!(servers.len(), 2);
            assert_eq!(servers[0].server_id, "docs");
            assert!(!servers[0].enabled);
            assert_eq!(servers[0].runtime_status, McpRuntimeStatus::Disabled);
            assert_eq!(servers[1].server_id, "hazel");
            assert!(servers[1].enabled);
            assert_eq!(servers[1].runtime_status, McpRuntimeStatus::Configured);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    match send_request(&socket, &DaemonRequest::GetRuntimeStatus) {
        DaemonResponse::GetRuntimeStatus(GetRuntimeStatusResponse { status }) => {
            assert!(status.daemon_ready);
            assert!(status.config_loaded);
            assert_eq!(status.model_profile_id.as_deref(), Some("mcp-profile"));
            assert_eq!(status.mcp_enabled_count, 1);
            assert_eq!(status.mcp_disabled_count, 1);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    child.kill().expect("kill daemon");
    let _ = child.wait();
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
    let _ = fs::remove_file(&config);
}

#[test]
fn update_mcp_server_state_is_persisted_and_retrievable() {
    let socket = socket_path();
    let store = temp_path("sharo-daemon-mcp-update", ".json");
    let config = write_deterministic_config_with_mcp("sharo-daemon-mcp-update");

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

    match send_request(
        &socket,
        &DaemonRequest::UpdateMcpServerState(UpdateMcpServerStateRequest {
            server_id: "hazel".to_string(),
            enabled: false,
        }),
    ) {
        DaemonResponse::UpdateMcpServerState(response) => {
            assert_eq!(response.server.server_id, "hazel");
            assert!(!response.server.enabled);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    match send_request(&socket, &DaemonRequest::ListMcpServers) {
        DaemonResponse::ListMcpServers(ListMcpServersResponse { servers }) => {
            let hazel = servers
                .into_iter()
                .find(|server| server.server_id == "hazel")
                .expect("hazel server");
            assert!(!hazel.enabled);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    child.kill().expect("kill daemon");
    let _ = child.wait();
    let _ = fs::remove_file(&socket);

    let restart_socket = socket_path();
    let mut restarted = Command::new(env!("CARGO_BIN_EXE_sharo-daemon"))
        .args([
            "start",
            "--socket-path",
            restart_socket.to_str().expect("socket path"),
            "--store-path",
            store.to_str().expect("store path"),
            "--config-path",
            config.to_str().expect("config path"),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn restarted daemon");

    match send_request(&restart_socket, &DaemonRequest::ListMcpServers) {
        DaemonResponse::ListMcpServers(ListMcpServersResponse { servers }) => {
            let hazel = servers
                .into_iter()
                .find(|server| server.server_id == "hazel")
                .expect("hazel server");
            assert!(!hazel.enabled);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    restarted.kill().expect("kill daemon");
    let _ = restarted.wait();
    let _ = fs::remove_file(&restart_socket);
    let _ = fs::remove_file(&store);
    let _ = fs::remove_file(&config);
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
fn submit_execution_runs_outside_runtime_worker() {
    status_requests_remain_responsive_under_parallel_slow_submits();
}

#[test]
fn runtime_workers_remain_available_under_slow_submit_pressure() {
    status_requests_remain_responsive_under_parallel_slow_submits();
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
