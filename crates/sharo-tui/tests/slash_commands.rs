mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use sharo_tui::app::{App, DaemonClient};
use sharo_tui::commands::{SlashCommand, parse_slash_command};
use support::{binary_path, temp_path, wait_for_socket};

fn create_skill(root: &Path, skill_id: &str, markdown: &str) {
    let skill_dir = root.join(skill_id);
    fs::create_dir_all(&skill_dir).expect("create skill dir");
    fs::write(skill_dir.join("SKILL.md"), markdown).expect("write skill");
}

fn write_slash_config(prefix: &str, skill_root: &Path) -> PathBuf {
    let config = temp_path(prefix, ".toml");
    fs::write(
        &config,
        format!(
            r#"[model]
provider = "deterministic"
model_id = "mock"
timeout_ms = 1000
profile_id = "slash-model"

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
    .expect("write slash config");
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
    let config = write_slash_config(prefix, &skill_root);
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

fn start_daemon_with_store_seed(
    prefix: &str,
    seeded_store: serde_json::Value,
) -> (std::process::Child, PathBuf, PathBuf, PathBuf, PathBuf) {
    let socket = temp_path(prefix, ".sock");
    let store = temp_path(prefix, ".json");
    let skill_root = temp_path(prefix, "-skills");
    fs::create_dir_all(&skill_root).expect("create skills root");
    create_skill(
        &skill_root,
        "writing/docs",
        "# Docs Writer\n\nStructured docs drafting support.\n",
    );
    fs::write(&store, seeded_store.to_string()).expect("write seeded store");
    let config = write_slash_config(prefix, &skill_root);
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

#[test]
fn slash_commands_dispatch_never_uses_chat_submit_path_for_control_actions() {
    let (daemon, socket, store, config, skill_root) = start_daemon("sharo-tui-slash-invariant");
    let mut app = App::new(DaemonClient::new(&socket));
    app.initialize().expect("initialize");

    let output = app
        .handle_chat_input("/sessions")
        .expect("sessions command");

    assert!(output.contains("sessions:"));
    assert!(app.state().active_session_id().is_none());
    assert!(app.state().sessions().is_empty());
    assert!(app.render_chat().contains("no active session"));

    cleanup(daemon, socket, store, config, skill_root);
}

#[test]
fn slash_commands_approve_command_resolves_pending_approval() {
    let (daemon, socket, store, config, skill_root) = start_daemon("sharo-tui-slash-approve");
    let mut app = App::new(DaemonClient::new(&socket));
    app.initialize().expect("initialize");
    app.create_session("beta").expect("create session");
    app.submit_turn("restricted: write secret").expect("submit");
    let approval_id = app.state().approvals()[0].approval_id.clone();

    let output = app
        .handle_chat_input(&format!("/approve {approval_id}"))
        .expect("approve command");

    assert!(output.contains("approval resolved"));
    assert!(!app.render_chat().contains("approval required:"));

    cleanup(daemon, socket, store, config, skill_root);
}

#[test]
fn slash_commands_session_switch_command_changes_active_session() {
    let (daemon, socket, store, config, skill_root) = start_daemon("sharo-tui-slash-switch");
    let mut app = App::new(DaemonClient::new(&socket));
    app.initialize().expect("initialize");
    let alpha = app.create_session("alpha").expect("alpha");
    app.submit_turn("read one context item")
        .expect("submit alpha");
    let beta = app.create_session("beta").expect("beta");

    let output = app
        .handle_chat_input(&format!("/session switch {alpha}"))
        .expect("switch command");

    assert!(output.contains(&alpha));
    assert_eq!(app.state().active_session_id(), Some(alpha.as_str()));
    assert_ne!(app.state().active_session_id(), Some(beta.as_str()));

    cleanup(daemon, socket, store, config, skill_root);
}

#[test]
fn slash_commands_skill_enable_command_updates_session_skill_state() {
    let (daemon, socket, store, config, skill_root) = start_daemon("sharo-tui-slash-skill");
    let mut app = App::new(DaemonClient::new(&socket));
    app.initialize().expect("initialize");
    app.create_session("alpha").expect("alpha");

    let output = app
        .handle_chat_input("/skill enable writing/docs")
        .expect("skill enable");

    assert!(output.contains("skill enabled"));
    let skills_output = app.handle_chat_input("/skills").expect("skills list");
    assert!(skills_output.contains("writing/docs [active]"));

    cleanup(daemon, socket, store, config, skill_root);
}

#[test]
fn slash_commands_mcp_listing_uses_stable_status_labels() {
    let (daemon, socket, store, config, skill_root) = start_daemon("sharo-tui-slash-mcp");
    let mut app = App::new(DaemonClient::new(&socket));
    app.initialize().expect("initialize");

    let before = app.handle_chat_input("/mcp").expect("mcp list");
    assert!(before.contains("hazel [enabled=true status=configured]"));

    let changed = app
        .handle_chat_input("/mcp disable hazel")
        .expect("mcp disable");
    assert!(changed.contains("enabled=false"));

    let after = app
        .handle_chat_input("/mcp")
        .expect("mcp list after disable");
    assert!(after.contains("hazel [enabled=false status=disabled]"));

    cleanup(daemon, socket, store, config, skill_root);
}

#[test]
fn slash_commands_parse_slash_command_with_argument_vector() {
    let parsed = parse_slash_command("/session switch session-42")
        .expect("parse")
        .expect("slash command");
    assert_eq!(
        parsed,
        SlashCommand::SessionSwitch {
            session_id: "session-42".to_string()
        }
    );
}

#[test]
fn slash_commands_parse_hazel_status_command() {
    let parsed = parse_slash_command("/hazel status")
        .expect("parse")
        .expect("slash command");
    assert_eq!(parsed, SlashCommand::HazelStatus);
}

#[test]
fn slash_commands_invalid_slash_command_returns_structured_error() {
    let error = parse_slash_command("/mcp enable a b").expect_err("invalid");
    assert_eq!(error.code, "slash_command_usage");
    assert!(error.message.contains("exactly one argument"));
}

#[test]
fn slash_commands_dispatch_errors_are_terminal_sanitized() {
    let (daemon, socket, store, config, skill_root) =
        start_daemon("sharo-tui-slash-error-sanitize");
    let mut app = App::new(DaemonClient::new(&socket));
    app.initialize().expect("initialize");

    let error = app
        .handle_chat_input("/session switch bad\u{1b}[31mid")
        .expect_err("unknown session should fail");

    assert!(error.contains("session_not_found"));
    assert!(!error.contains('\u{1b}'));
    assert!(error.contains("\\u{1b}"));

    cleanup(daemon, socket, store, config, skill_root);
}

#[test]
fn slash_commands_hazel_cards_command_switches_to_hazel_screen() {
    let (daemon, socket, store, config, skill_root) = start_daemon("sharo-tui-slash-hazel");
    let mut app = App::new(DaemonClient::new(&socket));
    app.initialize().expect("initialize");

    let output = app
        .handle_chat_input("/hazel cards")
        .expect("hazel cards");

    assert!(output.contains("hazel cards:"));
    assert_eq!(app.state().active_screen(), sharo_tui::state::Screen::Hazel);
    assert!(app.render_hazel().contains("hazel cards:"));

    cleanup(daemon, socket, store, config, skill_root);
}

#[test]
fn slash_commands_hazel_validate_and_enqueue_render_operator_results() {
    let (daemon, socket, store, config, skill_root) = start_daemon_with_store_seed(
        "sharo-tui-slash-hazel-actions",
        serde_json::json!({
            "hazel_proposal_batches": {
                "batch-000001": {
                    "batch_id": "batch-000001",
                    "idempotency_key": "idemp-000001",
                    "provenance": { "source_ref": "note:hazel", "producer": "operator" },
                    "proposals": [{
                        "proposal_id": "proposal-000001",
                        "kind": "chunk_upsert",
                        "chunk": {
                            "chunk_id": "chunk-000001",
                            "content": "hazel inspection batch",
                            "source_ref": "note:hazel"
                        },
                        "entity": null,
                        "relation": null,
                        "assertion": null
                    }]
                }
            },
            "hazel_sleep_jobs": {
                "job-000001": {
                "job_id": "job-000001",
                    "state": "pending",
                    "run_id": null,
                    "proposal_batch_ids": [],
                    "summary": "queued"
                }
            }
        }),
    )
    ;
    let mut app = App::new(DaemonClient::new(&socket));
    app.initialize().expect("initialize");

    let validate = app
        .handle_chat_input("/hazel validate batch-000001")
        .expect("hazel validate");
    assert!(validate.contains("hazel validate:"));

    let enqueue = app
        .handle_chat_input("/hazel enqueue-job note:operator job-001 \"user: remember hazel\"")
        .expect("hazel enqueue");
    assert!(enqueue.contains("hazel job:"));
    assert!(enqueue.contains("[completed]"));

    let cancel = app
        .handle_chat_input("/hazel cancel-job job-000001")
        .expect("hazel cancel");
    assert!(cancel.contains("hazel job:"));
    assert!(cancel.contains("[canceled]"));
    assert_eq!(app.state().active_screen(), sharo_tui::state::Screen::Hazel);

    cleanup(daemon, socket, store, config, skill_root);
}

#[test]
fn slash_commands_hazel_batches_lists_known_batch_ids() {
    let (daemon, socket, store, config, skill_root) = start_daemon_with_store_seed(
        "sharo-tui-slash-hazel-batches",
        serde_json::json!({
            "hazel_proposal_batches": {
                "batch-000001": {
                    "batch_id": "batch-000001",
                    "idempotency_key": "idemp-000001",
                    "provenance": { "source_ref": "note:hazel", "producer": "operator" },
                    "proposals": [{
                        "proposal_id": "proposal-000001",
                        "kind": "chunk_upsert",
                        "chunk": {
                            "chunk_id": "chunk-000001",
                            "content": "hazel inspection batch",
                            "source_ref": "note:hazel"
                        },
                        "entity": null,
                        "relation": null,
                        "assertion": null
                    }]
                }
            }
        }),
    );
    let mut app = App::new(DaemonClient::new(&socket));
    app.initialize().expect("initialize");

    let output = app
        .handle_chat_input("/hazel batches")
        .expect("hazel batches");

    assert!(output.contains("hazel batches:"));
    assert!(output.contains("batch-000001"));
    assert_eq!(app.state().active_screen(), sharo_tui::state::Screen::Hazel);

    cleanup(daemon, socket, store, config, skill_root);
}
