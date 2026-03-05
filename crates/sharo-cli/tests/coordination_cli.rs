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
fn cli_shows_coordination_summary_for_overlap() {
    let socket = unique_path("sharo-cli-coordination", ".sock");
    let store = unique_path("sharo-cli-coordination", ".json");

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
            "coordination-cli",
        ])
        .output()
        .expect("session open command");
    assert!(session.status.success());
    let session_id = String::from_utf8_lossy(&session.stdout)
        .trim()
        .trim_start_matches("session_id=")
        .to_string();

    let first = Command::new(env!("CARGO_BIN_EXE_sharo"))
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
            "draft scope:notes entry one",
        ])
        .output()
        .expect("first submit");
    assert!(first.status.success());

    let second = Command::new(env!("CARGO_BIN_EXE_sharo"))
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
            "draft scope:notes entry two",
        ])
        .output()
        .expect("second submit");
    assert!(second.status.success());

    let task_id = String::from_utf8_lossy(&second.stdout)
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
    let out = String::from_utf8_lossy(&get_task.stdout);
    assert!(out.contains("coordination_summary="));
    assert!(out.contains("scope=notes"));

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = std::fs::remove_file(socket);
    let _ = std::fs::remove_file(store);
}
