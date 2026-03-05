use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn unique_path(prefix: &str, suffix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("{}-{}{}", prefix, nanos, suffix))
}

fn daemon_bin() -> PathBuf {
    let cli_bin = PathBuf::from(env!("CARGO_BIN_EXE_sharo"));
    cli_bin.parent().expect("parent").join("sharo-daemon")
}

#[test]
fn approval_cli_parsing() {
    let output = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args(["approval", "--help"])
        .output()
        .expect("approval help command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("list"));
    assert!(stdout.contains("resolve"));
}

#[test]
fn approval_cli_resolve_is_idempotent_on_replay() {
    let socket = unique_path("sharo-cli-approval-idempotent", ".sock");
    let store = unique_path("sharo-cli-approval-idempotent", ".json");

    let mut daemon = Command::new(daemon_bin())
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket"),
            "--store-path",
            store.to_str().expect("store"),
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

    let open_session = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args([
            "--transport",
            "ipc",
            "--socket-path",
            socket.to_str().expect("socket"),
            "session",
            "open",
            "--label",
            "approval-cli",
        ])
        .output()
        .expect("session open command");
    assert!(open_session.status.success());
    let session_id = String::from_utf8_lossy(&open_session.stdout)
        .trim()
        .trim_start_matches("session_id=")
        .to_string();

    let submit = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args([
            "--transport",
            "ipc",
            "--socket-path",
            socket.to_str().expect("socket"),
            "task",
            "submit",
            "--session-id",
            &session_id,
            "--goal",
            "draft migration note",
        ])
        .output()
        .expect("task submit command");
    assert!(submit.status.success());

    let list = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args([
            "--transport",
            "ipc",
            "--socket-path",
            socket.to_str().expect("socket"),
            "approval",
            "list",
        ])
        .output()
        .expect("approval list command");
    assert!(list.status.success());
    let list_out = String::from_utf8_lossy(&list.stdout);
    let approval_id = list_out
        .split_whitespace()
        .find(|part| part.starts_with("approval_id="))
        .expect("approval_id")
        .trim_start_matches("approval_id=")
        .to_string();

    let resolve_first = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args([
            "--transport",
            "ipc",
            "--socket-path",
            socket.to_str().expect("socket"),
            "approval",
            "resolve",
            "--approval-id",
            &approval_id,
            "--decision",
            "deny",
        ])
        .output()
        .expect("approval resolve command");
    assert!(resolve_first.status.success());
    let first_out = String::from_utf8_lossy(&resolve_first.stdout);
    assert!(first_out.contains("state=denied"));

    let resolve_second = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args([
            "--transport",
            "ipc",
            "--socket-path",
            socket.to_str().expect("socket"),
            "approval",
            "resolve",
            "--approval-id",
            &approval_id,
            "--decision",
            "approve",
        ])
        .output()
        .expect("approval resolve replay command");
    assert!(resolve_second.status.success());
    let second_out = String::from_utf8_lossy(&resolve_second.stdout);
    assert!(second_out.contains("summary=idempotent_replay"));

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = std::fs::remove_file(socket);
    let _ = std::fs::remove_file(store);
}

#[test]
fn scenario_b_cli_blocked_and_approval_resolution() {
    let socket = unique_path("sharo-cli-scenario-b", ".sock");
    let store = unique_path("sharo-cli-scenario-b", ".json");

    let mut daemon = Command::new(daemon_bin())
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket"),
            "--store-path",
            store.to_str().expect("store"),
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

    let session = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args([
            "--transport",
            "ipc",
            "--socket-path",
            socket.to_str().expect("socket"),
            "session",
            "open",
            "--label",
            "scenario-b",
        ])
        .output()
        .expect("session open command");
    assert!(session.status.success());
    let session_id = String::from_utf8_lossy(&session.stdout)
        .trim()
        .trim_start_matches("session_id=")
        .to_string();

    let submit = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args([
            "--transport",
            "ipc",
            "--socket-path",
            socket.to_str().expect("socket"),
            "task",
            "submit",
            "--session-id",
            &session_id,
            "--goal",
            "draft release summary",
        ])
        .output()
        .expect("task submit command");
    assert!(submit.status.success());
    let submit_out = String::from_utf8_lossy(&submit.stdout);
    let task_id = submit_out
        .split_whitespace()
        .find(|part| part.starts_with("task_id="))
        .expect("task_id field")
        .trim_start_matches("task_id=")
        .to_string();

    let task_before = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args([
            "--transport",
            "ipc",
            "--socket-path",
            socket.to_str().expect("socket"),
            "task",
            "get",
            "--task-id",
            &task_id,
        ])
        .output()
        .expect("task get before approval");
    assert!(task_before.status.success());
    let task_before_out = String::from_utf8_lossy(&task_before.stdout);
    assert!(task_before_out.contains("task_state=awaiting_approval"));
    assert!(task_before_out.contains("blocking_reason=approval_required"));

    let list = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args([
            "--transport",
            "ipc",
            "--socket-path",
            socket.to_str().expect("socket"),
            "approval",
            "list",
        ])
        .output()
        .expect("approval list command");
    assert!(list.status.success());
    let list_out = String::from_utf8_lossy(&list.stdout);
    let approval_id = list_out
        .split_whitespace()
        .find(|part| part.starts_with("approval_id="))
        .expect("approval_id")
        .trim_start_matches("approval_id=")
        .to_string();

    let resolve = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args([
            "--transport",
            "ipc",
            "--socket-path",
            socket.to_str().expect("socket"),
            "approval",
            "resolve",
            "--approval-id",
            &approval_id,
            "--decision",
            "approve",
        ])
        .output()
        .expect("approval resolve command");
    assert!(resolve.status.success());
    let resolve_out = String::from_utf8_lossy(&resolve.stdout);
    assert!(resolve_out.contains("state=approved"));

    let task_after = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args([
            "--transport",
            "ipc",
            "--socket-path",
            socket.to_str().expect("socket"),
            "task",
            "get",
            "--task-id",
            &task_id,
        ])
        .output()
        .expect("task get after approval");
    assert!(task_after.status.success());
    let task_after_out = String::from_utf8_lossy(&task_after.stdout);
    assert!(task_after_out.contains("task_state=succeeded"));

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = std::fs::remove_file(socket);
    let _ = std::fs::remove_file(store);
}
