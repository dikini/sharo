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

fn write_deterministic_config(prefix: &str) -> PathBuf {
    let config = unique_path(prefix, ".toml");
    std::fs::write(
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

#[test]
fn cli_scenario_a_end_to_end() {
    let socket = unique_path("sharo-cli-scenario-a", ".sock");
    let store = unique_path("sharo-cli-scenario-a", ".json");
    let config = write_deterministic_config("sharo-cli-scenario-a");

    let mut daemon = Command::new(daemon_bin())
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket"),
            "--store-path",
            store.to_str().expect("store"),
            "--config-path",
            config.to_str().expect("config"),
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
    assert!(get_trace_out.contains("event_kind=fit_loop_fitted"));
    assert!(get_trace_out.contains("event_kind=model_output_received"));
    assert!(get_trace_out.contains("deterministic-response"));

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
    assert!(list_artifacts_out.contains("artifact_kind=fit_loop_decision"));
    assert!(list_artifacts_out.contains("artifact_kind=model_output"));
    assert!(list_artifacts_out.contains("deterministic-response"));

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();

    let _ = std::fs::remove_file(socket);
    let _ = std::fs::remove_file(store);
    let _ = std::fs::remove_file(config);
}

#[test]
fn cli_scenario_b_approval_commands() {
    let socket = unique_path("sharo-cli-scenario-b", ".sock");
    let store = unique_path("sharo-cli-scenario-b", ".json");
    let config = write_deterministic_config("sharo-cli-scenario-b");

    let mut daemon = Command::new(daemon_bin())
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket"),
            "--store-path",
            store.to_str().expect("store"),
            "--config-path",
            config.to_str().expect("config"),
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
            "restricted: write secret",
        ])
        .output()
        .expect("task submit command");
    assert!(submit.status.success());
    let submit_out = String::from_utf8_lossy(&submit.stdout);
    assert!(submit_out.contains("task_state=awaiting_approval"));
    let task_id = submit_out
        .split_whitespace()
        .find(|part| part.starts_with("task_id="))
        .expect("task_id field")
        .trim_start_matches("task_id=")
        .to_string();

    let task_get_before = Command::new(env!("CARGO_BIN_EXE_sharo"))
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
    assert!(task_get_before.status.success());
    let task_before_out = String::from_utf8_lossy(&task_get_before.stdout);
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
    assert!(list_out.contains("pending_approvals=1"));
    let approval_id = list_out
        .split_whitespace()
        .find(|part| part.starts_with("approval_id="))
        .expect("approval_id field")
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

    let task_get = Command::new(env!("CARGO_BIN_EXE_sharo"))
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
    assert!(task_get.status.success());
    let task_out = String::from_utf8_lossy(&task_get.stdout);
    assert!(task_out.contains("task_state=succeeded"));

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = std::fs::remove_file(socket);
    let _ = std::fs::remove_file(store);
    let _ = std::fs::remove_file(config);
}

#[test]
fn cli_scenario_c_overlap_is_visible_in_task_output() {
    let socket = unique_path("sharo-cli-scenario-c", ".sock");
    let store = unique_path("sharo-cli-scenario-c", ".json");
    let config = write_deterministic_config("sharo-cli-scenario-c");

    let mut daemon = Command::new(daemon_bin())
        .args([
            "start",
            "--socket-path",
            socket.to_str().expect("socket"),
            "--store-path",
            store.to_str().expect("store"),
            "--config-path",
            config.to_str().expect("config"),
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

    let session_1 = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args([
            "--transport",
            "ipc",
            "--socket-path",
            socket.to_str().expect("socket"),
            "session",
            "open",
            "--label",
            "s1",
        ])
        .output()
        .expect("session open command");
    let session_1_id = String::from_utf8_lossy(&session_1.stdout)
        .trim()
        .trim_start_matches("session_id=")
        .to_string();
    let session_2 = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args([
            "--transport",
            "ipc",
            "--socket-path",
            socket.to_str().expect("socket"),
            "session",
            "open",
            "--label",
            "s2",
        ])
        .output()
        .expect("session open command");
    let session_2_id = String::from_utf8_lossy(&session_2.stdout)
        .trim()
        .trim_start_matches("session_id=")
        .to_string();

    let _ = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args([
            "--transport",
            "ipc",
            "--socket-path",
            socket.to_str().expect("socket"),
            "task",
            "submit",
            "--session-id",
            &session_1_id,
            "--goal",
            "resource:alpha overlap check",
        ])
        .output()
        .expect("task submit command");

    let submit_2 = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args([
            "--transport",
            "ipc",
            "--socket-path",
            socket.to_str().expect("socket"),
            "task",
            "submit",
            "--session-id",
            &session_2_id,
            "--goal",
            "resource:alpha overlap check",
        ])
        .output()
        .expect("task submit command");
    assert!(submit_2.status.success());
    let task_2 = String::from_utf8_lossy(&submit_2.stdout)
        .split_whitespace()
        .find(|part| part.starts_with("task_id="))
        .expect("task id")
        .trim_start_matches("task_id=")
        .to_string();

    let task_get = Command::new(env!("CARGO_BIN_EXE_sharo"))
        .args([
            "--transport",
            "ipc",
            "--socket-path",
            socket.to_str().expect("socket"),
            "task",
            "get",
            "--task-id",
            &task_2,
        ])
        .output()
        .expect("task get command");
    assert!(task_get.status.success());
    let task_out = String::from_utf8_lossy(&task_get.stdout);
    assert!(task_out.contains("coordination_summary=conflict_detected"));

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = std::fs::remove_file(socket);
    let _ = std::fs::remove_file(store);
    let _ = std::fs::remove_file(config);
}
