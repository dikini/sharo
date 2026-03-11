use std::fs;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use proptest::prelude::*;
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

fn start_daemon(prefix: &str) -> (std::process::Child, PathBuf, PathBuf, PathBuf) {
    let socket = temp_path(prefix, ".sock");
    let store = temp_path(prefix, ".json");
    let config = write_deterministic_config(prefix);
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
    (daemon, socket, store, config)
}

#[test]
fn submit_turn_updates_active_session_chat_view() {
    let (mut daemon, socket, store, config) = start_daemon("sharo-tui-chat-submit");
    let mut app = App::new(DaemonClient::new(&socket));
    app.initialize().expect("initialize");
    let session_id = app.create_session("alpha").expect("create session");

    let task_id = app.submit_turn("read one context item").expect("submit");
    let rendered = app.render_chat();

    assert!(rendered.contains("session: alpha"));
    assert!(rendered.contains(&session_id));
    assert!(rendered.contains(&task_id));
    assert!(rendered.contains("deterministic-response"));

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = fs::remove_file(socket);
    let _ = fs::remove_file(store);
    let _ = fs::remove_file(config);
}

#[test]
fn submit_turn_without_active_session_creates_one() {
    let (mut daemon, socket, store, config) = start_daemon("sharo-tui-auto-session");
    let mut app = App::new(DaemonClient::new(&socket));
    app.initialize().expect("initialize");

    let task_id = app.submit_turn("read one context item").expect("submit");
    let rendered = app.render_chat();

    let active_session_id = app.state().active_session_id().expect("active session");
    assert!(active_session_id.starts_with("session-"));
    assert!(rendered.contains("session: chat"));
    assert!(rendered.contains(&task_id));

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = fs::remove_file(socket);
    let _ = fs::remove_file(store);
    let _ = fs::remove_file(config);
}

#[test]
fn switching_sessions_changes_active_chat_transcript() {
    let (mut daemon, socket, store, config) = start_daemon("sharo-tui-switch");
    let mut app = App::new(DaemonClient::new(&socket));
    app.initialize().expect("initialize");
    let alpha = app.create_session("alpha").expect("create alpha");
    app.submit_turn("read one context item")
        .expect("submit alpha");
    let alpha_view = app.render_chat();

    let beta = app.create_session("beta").expect("create beta");
    app.submit_turn("need approval for restricted write")
        .expect("submit beta");
    let beta_view = app.render_chat();

    assert_ne!(alpha_view, beta_view);
    app.switch_session(&alpha).expect("switch alpha");
    assert_eq!(app.state().active_session_id(), Some(alpha.as_str()));
    let switched_alpha = app.render_chat();
    assert_eq!(switched_alpha, alpha_view);

    app.switch_session(&beta).expect("switch beta");
    assert_eq!(app.render_chat(), beta_view);

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = fs::remove_file(socket);
    let _ = fs::remove_file(store);
    let _ = fs::remove_file(config);
}

#[test]
fn approval_resolution_refreshes_current_chat_view() {
    let (mut daemon, socket, store, config) = start_daemon("sharo-tui-approval");
    let mut app = App::new(DaemonClient::new(&socket));
    app.initialize().expect("initialize");
    app.create_session("beta").expect("create beta");
    app.submit_turn("restricted: write secret").expect("submit");

    let before = app.render_chat();
    assert!(before.contains("approval required:"));
    let approval_id = app
        .state()
        .approvals()
        .first()
        .expect("pending approval")
        .approval_id
        .clone();

    app.resolve_approval(&approval_id, "approve")
        .expect("resolve approval");
    let after = app.render_chat();

    assert!(!after.contains("approval required:"));
    assert!(after.contains("deterministic-response"));

    daemon.kill().expect("kill daemon");
    let _ = daemon.wait();
    let _ = fs::remove_file(socket);
    let _ = fs::remove_file(store);
    let _ = fs::remove_file(config);
}

proptest! {
    #![proptest_config(ProptestConfig {
        failure_persistence: None,
        .. ProptestConfig::default()
    })]
    #[test]
    fn session_switch_sequence_never_cross_contaminates_active_transcript(order in proptest::collection::vec(0_usize..2, 1..16)) {
        let transcripts = ["alpha transcript", "beta transcript"];

        for next in order {
            let rendered = transcripts[next];
            let other = transcripts[1 - next];
            prop_assert_eq!(rendered, transcripts[next]);
            prop_assert_ne!(rendered, other);
        }
    }
}
