use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn socket_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("sharo-cli-test-{}.sock", nanos))
}

fn daemon_bin() -> PathBuf {
    let cli_bin = PathBuf::from(env!("CARGO_BIN_EXE_sharo"));
    cli_bin
        .parent()
        .expect("cli binary parent")
        .join("sharo-daemon")
}

#[test]
fn cli_submit_and_status_stub_smoke() {
    let submit = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args(["--transport", "stub", "submit", "--goal", "read docs"])
        .output()
        .expect("submit command should run");

    assert!(submit.status.success());
    let submit_stdout = String::from_utf8_lossy(&submit.stdout);
    assert!(submit_stdout.contains("task_id="));
    assert!(submit_stdout.contains("state=Submitted"));

    let status = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args(["--transport", "stub", "status", "--task-id", "task-0001"])
        .output()
        .expect("status command should run");

    assert!(status.status.success());
    let status_stdout = String::from_utf8_lossy(&status.stdout);
    assert!(status_stdout.contains("task_id=task-0001"));
    assert!(status_stdout.contains("state="));
    assert!(status_stdout.contains("summary="));
}

#[test]
fn cli_submit_status_against_daemon_socket() {
    let socket = socket_path();

    let mut daemon_submit = Command::new(daemon_bin())
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("path"),
            "--serve-once",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon submit");

    for _ in 0..50 {
        if socket.exists() {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }

    let submit = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args([
            "--transport",
            "ipc",
            "--socket-path",
            socket.to_str().expect("path"),
            "submit",
            "--goal",
            "read docs",
        ])
        .output()
        .expect("submit over ipc");

    assert!(submit.status.success());
    let submit_stdout = String::from_utf8_lossy(&submit.stdout);
    assert!(submit_stdout.contains("task_id="));
    assert!(submit_stdout.contains("state=Submitted"));

    assert!(daemon_submit.wait().expect("wait daemon submit").success());

    let mut daemon_status = Command::new(daemon_bin())
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("path"),
            "--serve-once",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon status");

    for _ in 0..50 {
        if socket.exists() {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }

    let status = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args([
            "--transport",
            "ipc",
            "--socket-path",
            socket.to_str().expect("path"),
            "status",
            "--task-id",
            "task-0001",
        ])
        .output()
        .expect("status over ipc");

    assert!(status.status.success());
    let status_stdout = String::from_utf8_lossy(&status.stdout);
    assert!(status_stdout.contains("task_id=task-0001"));
    assert!(status_stdout.contains("state="));
    assert!(status_stdout.contains("summary="));

    assert!(daemon_status.wait().expect("wait daemon status").success());
}

#[test]
fn cli_hazel_status_against_daemon_socket() {
    let socket = socket_path();

    let mut daemon = Command::new(daemon_bin())
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("path"),
            "--serve-once",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    for _ in 0..50 {
        if socket.exists() {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }

    let status = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args([
            "--transport",
            "ipc",
            "--socket-path",
            socket.to_str().expect("path"),
            "hazel",
            "status",
        ])
        .output()
        .expect("hazel status over ipc");

    assert!(status.status.success());
    let stdout = String::from_utf8_lossy(&status.stdout);
    assert!(stdout.contains("available=true"));
    assert!(stdout.contains("cards="));

    assert!(daemon.wait().expect("wait daemon").success());
}

#[test]
fn cli_hazel_validate_and_enqueue_against_daemon_socket() {
    let socket = socket_path();
    let store = std::env::temp_dir().join(format!(
        "sharo-cli-hazel-store-{}.json",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    std::fs::write(
        &store,
        serde_json::json!({
            "hazel_proposal_batches": {
                "batch-000001": {
                    "batch_id": "batch-000001",
                    "idempotency_key": "idemp-000001",
                    "provenance": { "source_ref": "note:hazel", "producer": "operator" },
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
            }
        })
        .to_string(),
    )
    .expect("write store");

    let mut daemon = Command::new(daemon_bin())
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("path"),
            "--store-path",
            store.to_str().expect("store"),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    for _ in 0..50 {
        if socket.exists() {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }

    let validate = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args([
            "--transport",
            "ipc",
            "--socket-path",
            socket.to_str().expect("path"),
            "hazel",
            "validate",
            "--batch-id",
            "batch-000001",
        ])
        .output()
        .expect("hazel validate over ipc");
    assert!(validate.status.success());
    let validate_stdout = String::from_utf8_lossy(&validate.stdout);
    assert!(validate_stdout.contains("validation_id="));
    assert!(validate_stdout.contains("accepted=true"));

    let enqueue = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args([
            "--transport",
            "ipc",
            "--socket-path",
            socket.to_str().expect("path"),
            "hazel",
            "enqueue-job",
            "--source-ref",
            "note:operator",
            "--idempotency-key",
            "job-001",
            "--message",
            "user: remember hazel",
        ])
        .output()
        .expect("hazel enqueue over ipc");
    assert!(enqueue.status.success());
    let enqueue_stdout = String::from_utf8_lossy(&enqueue.stdout);
    assert!(enqueue_stdout.contains("job_id=hazel-job-job-001"));
    assert!(enqueue_stdout.contains("state=completed"));

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = std::fs::remove_file(&socket);
    let _ = std::fs::remove_file(&store);
}
