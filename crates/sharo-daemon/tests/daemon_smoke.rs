use std::process::Command;
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_path(prefix: &str, suffix: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{nanos}{suffix}"))
}

#[test]
fn daemon_start_smoke() {
    let output = Command::new(env!("CARGO_BIN_EXE_sharo-daemon"))
        .args(["start", "--once"])
        .output()
        .expect("daemon command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("daemon_started"));
    assert!(stdout.contains("daemon_stopped"));
}

#[test]
fn daemon_start_rejects_invalid_mcp_config() {
    let config = temp_path("sharo-daemon-invalid-mcp", ".toml");
    let socket = temp_path("sharo-daemon-invalid-mcp", ".sock");
    let store = temp_path("sharo-daemon-invalid-mcp", ".json");
    fs::write(
        &config,
        r#"[model]
provider = "deterministic"
model_id = "mock"

[[mcp.servers]]
server_id = "hazel"
transport = "stdio"
"#,
    )
    .expect("write config");

    let output = Command::new(env!("CARGO_BIN_EXE_sharo-daemon"))
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket path"),
            "--store-path",
            store.to_str().expect("store path"),
            "--config-path",
            config.to_str().expect("config path"),
        ])
        .output()
        .expect("daemon command should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("daemon_error=mcp_config_invalid"));

    let _ = fs::remove_file(&config);
    let _ = fs::remove_file(&socket);
    let _ = fs::remove_file(&store);
}
