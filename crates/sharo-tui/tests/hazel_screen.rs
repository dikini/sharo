mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use sharo_tui::app::{App, DaemonClient};
use sharo_tui::state::Screen;
use support::{binary_path, temp_path, wait_for_socket};

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

[skills]
roots = ["{}"]
enable_project_skills = false
enable_user_skills = false
trust_project_skills = true
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
    create_skill(&skill_root, "writing/docs", "# Docs Writer\n");
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
fn hazel_screen_renders_latest_hazel_panel() {
    let (daemon, socket, store, config, skill_root) = start_daemon("sharo-tui-hazel-screen");
    let mut app = App::new(DaemonClient::new(&socket));
    app.initialize().expect("initialize");

    let output = app
        .handle_chat_input("/hazel status")
        .expect("hazel status");

    assert!(output.contains("hazel status:"));
    assert_eq!(app.state().active_screen(), Screen::Hazel);
    let shell = app.render_shell();
    assert!(shell.contains("screen: Hazel"));
    assert!(shell.contains("hazel status:"));

    cleanup(daemon, socket, store, config, skill_root);
}
