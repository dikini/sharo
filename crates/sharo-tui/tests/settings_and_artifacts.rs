use std::fs;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sharo_tui::app::{App, DaemonClient};

fn temp_path(prefix: &str, suffix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{nanos}{suffix}"))
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

fn create_skill(root: &Path, skill_id: &str, markdown: &str) {
    let skill_dir = root.join(skill_id);
    fs::create_dir_all(&skill_dir).expect("create skill dir");
    fs::write(skill_dir.join("SKILL.md"), markdown).expect("write skill");
}

fn write_config(prefix: &str, skill_root: &Path) -> PathBuf {
    let config = temp_path(prefix, ".toml");
    fs::write(
        &config,
        format!(
            r#"[model]
provider = "deterministic"
model_id = "mock"
timeout_ms = 1000
profile_id = "settings-model"

[skills]
roots = ["{}"]
enable_project_skills = false
enable_user_skills = false
trust_project_skills = true

[[mcp.servers]]
server_id = "hazel"
display_name = "Hazel"
transport = "stdio"
command = "/usr/bin/hazel-mcp"
enabled = true
"#,
            skill_root.display()
        ),
    )
    .expect("write config");
    config
}

fn start_daemon(prefix: &str) -> (std::process::Child, PathBuf, PathBuf, PathBuf, PathBuf) {
    let socket = temp_path(prefix, ".sock");
    let store = temp_path(prefix, ".json");
    let skill_root = temp_path(prefix, "-skills");
    fs::create_dir_all(&skill_root).expect("create skills root");
    create_skill(
        &skill_root,
        "writing/docs",
        "# Docs Writer\n\nStructured docs drafting support.\n",
    );
    let config = write_config(prefix, &skill_root);
    let daemon = Command::new(binary_path("sharo-daemon"))
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
    wait_for_socket(&socket);
    (daemon, socket, store, config, skill_root)
}

fn cleanup(
    mut daemon: std::process::Child,
    socket: PathBuf,
    store: PathBuf,
    config: PathBuf,
    skill_root: PathBuf,
) {
    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = fs::remove_file(socket);
    let _ = fs::remove_file(store);
    let _ = fs::remove_file(config);
    let _ = fs::remove_dir_all(skill_root);
}

#[test]
fn settings_and_artifacts_settings_screen_renders_skill_and_mcp_status() {
    let (daemon, socket, store, config, skill_root) = start_daemon("sharo-tui-settings");
    let mut app = App::new(DaemonClient::new(&socket));
    app.initialize().expect("initialize");
    app.create_session("alpha").expect("create session");
    app.handle_chat_input("/skill enable writing/docs")
        .expect("enable skill");

    let rendered = app.render_settings();

    assert!(rendered.contains("model profile: settings-model"));
    assert!(rendered.contains("skills:"));
    assert!(rendered.contains("writing/docs [active]"));
    assert!(rendered.contains("mcp servers:"));
    assert!(rendered.contains("hazel [enabled=true status=configured]"));

    cleanup(daemon, socket, store, config, skill_root);
}

#[test]
fn settings_and_artifacts_artifact_screen_renders_route_and_final_result_records() {
    let (daemon, socket, store, config, skill_root) = start_daemon("sharo-tui-artifacts");
    let mut app = App::new(DaemonClient::new(&socket));
    app.initialize().expect("initialize");
    app.create_session("alpha").expect("create session");
    app.submit_turn("read one context item").expect("submit");

    let rendered = app.render_trace_artifacts();

    assert!(rendered.contains("trace:"));
    assert!(rendered.contains("route_decision"));
    assert!(rendered.contains("artifacts:"));
    assert!(rendered.contains("final_result"));

    cleanup(daemon, socket, store, config, skill_root);
}

#[test]
fn settings_and_artifacts_failed_session_switch_preserves_previous_cached_views() {
    let (mut daemon, socket, store, config, skill_root) =
        start_daemon("sharo-tui-switch-regression");
    let mut app = App::new(DaemonClient::new(&socket));
    app.initialize().expect("initialize");
    let alpha = app.create_session("alpha").expect("create alpha");
    app.submit_turn("read one context item").expect("submit alpha");
    let beta = app.create_session("beta").expect("create beta");
    app.switch_session(&alpha).expect("switch to alpha");

    let expected_chat = app.render_chat();
    let expected_artifacts = app.render_trace_artifacts();

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();

    let error = app.switch_session(&beta).expect_err("switch should fail");
    assert!(error.contains("daemon_connect_failed"));
    assert_eq!(app.state().active_session_id(), Some(alpha.as_str()));
    assert_eq!(app.render_chat(), expected_chat);
    assert_eq!(app.render_trace_artifacts(), expected_artifacts);

    let _ = fs::remove_file(socket);
    let _ = fs::remove_file(store);
    let _ = fs::remove_file(config);
    let _ = fs::remove_dir_all(skill_root);
}

#[test]
fn settings_and_artifacts_failed_refresh_sessions_preserves_previous_cached_views() {
    let (mut daemon, socket, store, config, skill_root) =
        start_daemon("sharo-tui-refresh-regression");
    let mut app = App::new(DaemonClient::new(&socket));
    app.initialize().expect("initialize");
    let alpha = app.create_session("alpha").expect("create alpha");
    app.submit_turn("read one context item").expect("submit alpha");

    let expected_chat = app.render_chat();
    let expected_artifacts = app.render_trace_artifacts();
    let expected_settings = app.render_settings();

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();

    let error = app.refresh_sessions().expect_err("refresh should fail");
    assert!(error.contains("daemon_connect_failed"));
    assert_eq!(app.state().active_session_id(), Some(alpha.as_str()));
    assert_eq!(app.render_chat(), expected_chat);
    assert_eq!(app.render_trace_artifacts(), expected_artifacts);
    assert_eq!(app.render_settings(), expected_settings);

    let _ = fs::remove_file(socket);
    let _ = fs::remove_file(store);
    let _ = fs::remove_file(config);
    let _ = fs::remove_dir_all(skill_root);
}
