use std::process::Command;

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
