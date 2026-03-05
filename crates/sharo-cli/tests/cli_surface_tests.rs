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
    let build = Command::new("cargo")
        .args(["build", "--package", "sharo-daemon", "--bin", "sharo-daemon"])
        .output()
        .expect("build daemon binary");
    assert!(build.status.success());

    let cli_bin = PathBuf::from(env!("CARGO_BIN_EXE_sharo"));
    cli_bin.parent().expect("parent").join("sharo-daemon")
}

#[test]
fn cli_surface_has_required_command_groups() {
    let output = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .arg("--help")
        .output()
        .expect("help command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    for group in ["session", "task", "trace", "artifacts", "approval", "daemon"] {
        assert!(stdout.contains(group), "missing command group: {group}");
    }
}

#[test]
fn cli_read_commands_are_side_effect_free() {
    let socket = unique_path("sharo-cli-surface-read", ".sock");
    let store = unique_path("sharo-cli-surface-read", ".json");

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

    let before = std::fs::metadata(&store).ok().and_then(|m| m.modified().ok());

    let task_get = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args([
            "--transport",
            "ipc",
            "--socket-path",
            socket.to_str().expect("socket"),
            "task",
            "get",
            "--task-id",
            "task-does-not-exist",
        ])
        .output()
        .expect("task get");
    assert!(!task_get.status.success());

    let daemon_ping = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args([
            "--transport",
            "ipc",
            "--socket-path",
            socket.to_str().expect("socket"),
            "daemon",
            "ping",
        ])
        .output()
        .expect("daemon ping");
    assert!(daemon_ping.status.success());

    let after = std::fs::metadata(&store).ok().and_then(|m| m.modified().ok());
    assert_eq!(before, after);

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = std::fs::remove_file(socket);
    let _ = std::fs::remove_file(store);
}

#[test]
fn cli_command_set_maps_to_protocol_operations() {
    let socket = unique_path("sharo-cli-surface-map", ".sock");
    let store = unique_path("sharo-cli-surface-map", ".json");

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
            "cli-surface",
        ])
        .output()
        .expect("session open");
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
            "--idempotency-key",
            "idem-42",
            "--goal",
            "read scope:docs one page",
        ])
        .output()
        .expect("task submit");
    assert!(submit.status.success());
    let submit_out = String::from_utf8_lossy(&submit.stdout);
    let task_id = submit_out
        .split_whitespace()
        .find(|part| part.starts_with("task_id="))
        .expect("task id")
        .trim_start_matches("task_id=")
        .to_string();

    let cancel = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args([
            "--transport",
            "ipc",
            "--socket-path",
            socket.to_str().expect("socket"),
            "task",
            "cancel",
            "--task-id",
            &task_id,
        ])
        .output()
        .expect("task cancel");
    assert!(cancel.status.success());
    let cancel_out = String::from_utf8_lossy(&cancel.stdout);
    assert!(cancel_out.contains("accepted=true"));

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = std::fs::remove_file(socket);
    let _ = std::fs::remove_file(store);
}
