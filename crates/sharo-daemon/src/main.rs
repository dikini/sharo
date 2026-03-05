use std::path::PathBuf;

use clap::{Parser, Subcommand};
use sharo_core::client::{RuntimeClient, StubClient};
use sharo_core::protocol::{
    DaemonInfoResponse, DaemonRequest, DaemonResponse, GetArtifactsResponse, GetTaskResponse, GetTraceResponse,
    ListPendingApprovalsResponse, ListTasksResponse, RegisterSessionResponse, SubmitTaskOpResponse,
    TaskStatusRequest,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

mod store;
use store::Store;

const DEFAULT_SOCKET_PATH: &str = "/tmp/sharo-daemon.sock";
const DEFAULT_STORE_PATH: &str = "/tmp/sharo-daemon-store.json";

#[derive(Debug, Parser)]
#[command(name = "sharo-daemon")]
#[command(about = "Sharo daemon")]
struct Cli {
    #[command(subcommand)]
    command: DaemonCommand,
}

#[derive(Debug, Subcommand)]
enum DaemonCommand {
    Start {
        #[arg(long)]
        once: bool,
        #[arg(long, default_value = DEFAULT_SOCKET_PATH)]
        socket_path: PathBuf,
        #[arg(long, default_value = DEFAULT_STORE_PATH)]
        store_path: PathBuf,
        #[arg(long)]
        serve_once: bool,
    },
}

fn handle_request(request: DaemonRequest, client: &impl RuntimeClient, store: &mut Store) -> DaemonResponse {
    match request {
        DaemonRequest::Submit(submit) => DaemonResponse::Submit(client.submit(&submit)),
        DaemonRequest::Status(status) => DaemonResponse::Status(client.status(&TaskStatusRequest {
            task_id: status.task_id,
        })),
        DaemonRequest::RegisterSession(payload) => match store.register_session(&payload.session_label) {
            Ok(session_id) => DaemonResponse::RegisterSession(RegisterSessionResponse { session_id }),
            Err(message) => DaemonResponse::Error { message },
        },
        DaemonRequest::SubmitTask(payload) => match store.submit_task(payload) {
            Ok(SubmitTaskOpResponse {
                task_id,
                task_state,
                accepted,
                reason,
                summary,
            }) => DaemonResponse::SubmitTask(SubmitTaskOpResponse {
                task_id,
                task_state,
                accepted,
                reason,
                summary,
            }),
            Err(message) => DaemonResponse::Error { message },
        },
        DaemonRequest::ControlTask(payload) => match store.control_task(&payload.task_id, &payload.action) {
            Ok(response) => DaemonResponse::ControlTask(response),
            Err(message) => DaemonResponse::Error { message },
        },
        DaemonRequest::GetTask(payload) => match store.get_task(&payload.task_id) {
            Some(task) => DaemonResponse::GetTask(GetTaskResponse { task }),
            None => DaemonResponse::Error {
                message: format!("task_not_found task_id={}", payload.task_id),
            },
        },
        DaemonRequest::ListTasks(_) => DaemonResponse::ListTasks(ListTasksResponse {
            tasks: store.list_tasks(),
        }),
        DaemonRequest::GetTrace(payload) => match store.get_trace(&payload.task_id) {
            Some(trace) => DaemonResponse::GetTrace(GetTraceResponse { trace }),
            None => DaemonResponse::Error {
                message: format!("trace_not_found task_id={}", payload.task_id),
            },
        },
        DaemonRequest::GetArtifacts(payload) => DaemonResponse::GetArtifacts(GetArtifactsResponse {
            artifacts: store.get_artifacts(&payload.task_id),
        }),
        DaemonRequest::ListPendingApprovals(_) => {
            DaemonResponse::ListPendingApprovals(ListPendingApprovalsResponse {
                approvals: store.list_pending_approvals(),
            })
        }
        DaemonRequest::ResolveApproval(payload) => match store.resolve_approval(
            &payload.approval_id,
            &payload.decision,
        ) {
            Ok(response) => DaemonResponse::ResolveApproval(response),
            Err(message) => DaemonResponse::Error { message },
        },
        DaemonRequest::DaemonInfo(_) => DaemonResponse::DaemonInfo(DaemonInfoResponse {
            daemon_state: "ready".to_string(),
            summary: "daemon reachable".to_string(),
        }),
    }
}

async fn handle_stream(stream: UnixStream, client: &impl RuntimeClient, store: &mut Store) {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let mut line = String::new();
    match reader.read_line(&mut line).await {
        Ok(0) => {
            let _ = writer
                .write_all(b"{\"Error\":{\"message\":\"empty request\"}}\n")
                .await;
        }
        Ok(_) => {
            let response = match serde_json::from_str::<DaemonRequest>(line.trim()) {
                Ok(request) => handle_request(request, client, store),
                Err(error) => DaemonResponse::Error {
                    message: format!("invalid request: {error}"),
                },
            };

            match serde_json::to_string(&response) {
                Ok(payload) => {
                    let _ = writer.write_all(payload.as_bytes()).await;
                    let _ = writer.write_all(b"\n").await;
                }
                Err(error) => {
                    let _ = writer
                        .write_all(
                            format!(
                                "{{\"Error\":{{\"message\":\"serialization failure: {}\"}}}}\n",
                                error
                            )
                            .as_bytes(),
                        )
                        .await;
                }
            }
        }
        Err(error) => {
            let _ = writer
                .write_all(format!("{{\"Error\":{{\"message\":\"read failure: {}\"}}}}\n", error).as_bytes())
                .await;
        }
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        DaemonCommand::Start {
            once,
            socket_path,
            store_path,
            serve_once,
        } => {
            println!("daemon_started");

            if once {
                println!("daemon_stopped");
                return;
            }

            let _ = std::fs::remove_file(&socket_path);
            let listener = match UnixListener::bind(&socket_path) {
                Ok(listener) => listener,
                Err(error) => {
                    eprintln!("daemon_error=bind_failed message={}", error);
                    std::process::exit(1);
                }
            };

            let mut store = match Store::open(&store_path) {
                Ok(store) => store,
                Err(message) => {
                    eprintln!("daemon_error=store_open_failed message={}", message);
                    std::process::exit(1);
                }
            };

            let client = StubClient;

            loop {
                tokio::select! {
                    accept_result = listener.accept() => {
                        let (stream, _) = match accept_result {
                            Ok(pair) => pair,
                            Err(error) => {
                                eprintln!("daemon_error=accept_failed message={}", error);
                                continue;
                            }
                        };

                        handle_stream(stream, &client, &mut store).await;

                        if serve_once {
                            break;
                        }
                    }
                    _ = tokio::signal::ctrl_c() => {
                        break;
                    }
                }
            }

            let _ = std::fs::remove_file(&socket_path);
            println!("daemon_stopped");
        }
    }
}
