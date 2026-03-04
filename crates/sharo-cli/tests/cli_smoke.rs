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
