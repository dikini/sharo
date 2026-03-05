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
fn cli_scenario_a_end_to_end() {
    let socket = unique_path("sharo-cli-scenario-a", ".sock");
    let store = unique_path("sharo-cli-scenario-a", ".json");

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
            "scenario-a",
        ])
        .output()
        .expect("session open command");
    assert!(session.status.success());
    let session_out = String::from_utf8_lossy(&session.stdout);
    assert!(session_out.contains("session_id="));
    let session_id = session_out.trim().trim_start_matches("session_id=").to_string();

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
            "read one context item",
        ])
        .output()
        .expect("task submit command");
    assert!(submit.status.success());
    let submit_out = String::from_utf8_lossy(&submit.stdout);
    assert!(submit_out.contains("task_id="));

    let task_id = submit_out
        .split_whitespace()
        .find(|part| part.starts_with("task_id="))
        .expect("task_id field")
        .trim_start_matches("task_id=")
        .to_string();

    let get_task = Command::new(env!("CARGO_BIN_EXE_sharo"))
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
        .expect("task get command");
    assert!(get_task.status.success());
    let get_task_out = String::from_utf8_lossy(&get_task.stdout);
    assert!(get_task_out.contains("task_state=succeeded"));

    let get_trace = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args([
            "--transport",
            "ipc",
            "--socket-path",
            socket.to_str().expect("socket"),
            "trace",
            "get",
            "--task-id",
            &task_id,
        ])
        .output()
        .expect("trace get command");
    assert!(get_trace.status.success());
    let get_trace_out = String::from_utf8_lossy(&get_trace.stdout);
    assert!(get_trace_out.contains("events="));

    let list_artifacts = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args([
            "--transport",
            "ipc",
            "--socket-path",
            socket.to_str().expect("socket"),
            "artifacts",
            "list",
            "--task-id",
            &task_id,
        ])
        .output()
        .expect("artifacts list command");
    assert!(list_artifacts.status.success());
    let list_artifacts_out = String::from_utf8_lossy(&list_artifacts.stdout);
    assert!(list_artifacts_out.contains("artifacts="));

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();

    let _ = std::fs::remove_file(socket);
    let _ = std::fs::remove_file(store);
}
