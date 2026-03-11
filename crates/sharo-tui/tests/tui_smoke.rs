use std::fs;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn temp_path(prefix: &str, suffix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{nanos}{suffix}"))
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

fn binary_path(name: &str) -> PathBuf {
    let current = std::env::current_exe().expect("current exe");
    let debug_dir = current
        .parent()
        .and_then(|path| path.parent())
        .expect("target debug dir");
    debug_dir.join(name)
}

fn wait_for_socket(socket: &PathBuf) {
    for _ in 0..80 {
        if UnixStream::connect(socket).is_ok() {
            return;
        }
        thread::sleep(Duration::from_millis(15));
    }
    panic!("connect to daemon socket");
}

#[test]
fn tui_starts_and_renders_chat_shell() {
    let socket = temp_path("sharo-tui-smoke", ".sock");
    let store = temp_path("sharo-tui-smoke", ".json");
    let config = write_deterministic_config("sharo-tui-smoke");

    let mut daemon = Command::new(binary_path("sharo-daemon"))
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

    wait_for_socket(&socket);

    let output = Command::new(binary_path("sharo-tui"))
        .args([
            "--socket-path",
            socket.to_str().expect("socket path"),
            "--once",
        ])
        .output()
        .expect("run tui");

    assert!(
        output.status.success(),
        "tui stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Sharo TUI"));
    assert!(stdout.contains("screen: Chat"));
    assert!(stdout.contains("daemon: connected"));

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
    let _ = fs::remove_file(&config);
}

#[test]
fn tui_can_start_on_non_default_screen() {
    let socket = temp_path("sharo-tui-screen", ".sock");
    let store = temp_path("sharo-tui-screen", ".json");
    let config = write_deterministic_config("sharo-tui-screen");

    let mut daemon = Command::new(binary_path("sharo-daemon"))
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

    wait_for_socket(&socket);

    let output = Command::new(binary_path("sharo-tui"))
        .args([
            "--socket-path",
            socket.to_str().expect("socket path"),
            "--screen",
            "settings",
            "--once",
        ])
        .output()
        .expect("run tui");

    assert!(
        output.status.success(),
        "tui stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("screen: Settings"));

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
    let _ = fs::remove_file(&config);
}
