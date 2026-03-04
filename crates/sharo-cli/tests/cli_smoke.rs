use std::process::Command;

#[test]
fn cli_submit_and_status_smoke() {
    let submit = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args(["submit", "--goal", "read docs"])
        .output()
        .expect("submit command should run");

    assert!(submit.status.success());
    let submit_stdout = String::from_utf8_lossy(&submit.stdout);
    assert!(submit_stdout.contains("task_id="));
    assert!(submit_stdout.contains("state=Submitted"));

    let status = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args(["status", "--task-id", "task-0001"])
        .output()
        .expect("status command should run");

    assert!(status.status.success());
    let status_stdout = String::from_utf8_lossy(&status.stdout);
    assert!(status_stdout.contains("task_id=task-0001"));
    assert!(status_stdout.contains("state="));
    assert!(status_stdout.contains("summary="));
}
