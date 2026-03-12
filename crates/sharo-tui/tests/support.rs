use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn temp_path(prefix: &str, suffix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{nanos}{suffix}"))
}

pub fn binary_path(name: &str) -> PathBuf {
    static DAEMON_BIN: OnceLock<PathBuf> = OnceLock::new();
    static TUI_BIN: OnceLock<PathBuf> = OnceLock::new();
    let cache = match name {
        "sharo-daemon" => &DAEMON_BIN,
        "sharo-tui" => &TUI_BIN,
        other => panic!("unsupported binary lookup: {other}"),
    };
    if let Some(path) = cache.get() {
        return path.clone();
    }

    let current = std::env::current_exe().expect("current exe");
    let debug_dir = current
        .parent()
        .and_then(|path| path.parent())
        .expect("target debug dir");
    let path = debug_dir.join(name);
    if path.exists() {
        let _ = cache.set(path.clone());
        return path;
    }

    let package = match name {
        "sharo-daemon" => "sharo-daemon",
        "sharo-tui" => "sharo-tui",
        other => panic!("unsupported binary lookup: {other}"),
    };
    let status = Command::new(env!("CARGO"))
        .args(["build", "-p", package, "--bin", name])
        .status()
        .expect("build binary");
    assert!(status.success(), "build {name} must succeed");
    assert!(path.exists(), "expected built binary at {}", path.display());
    let _ = cache.set(path.clone());
    path
}

pub fn wait_for_socket(socket: &PathBuf) {
    for _ in 0..80 {
        if UnixStream::connect(socket).is_ok() {
            return;
        }
        thread::sleep(Duration::from_millis(15));
    }
    panic!("connect to daemon socket");
}
