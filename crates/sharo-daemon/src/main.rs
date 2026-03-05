use std::path::PathBuf;
#[cfg(unix)]
use std::{fs, os::unix::fs::PermissionsExt};

use clap::{Parser, Subcommand};
use sharo_core::client::{RuntimeClient, StubClient};
use sharo_core::protocol::{
    DaemonRequest, DaemonResponse, GetArtifactsResponse, GetTaskResponse, GetTraceResponse,
    RegisterSessionResponse, SubmitTaskOpResponse, TaskStatusRequest,
};
use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};

mod store;
use store::Store;

const DEFAULT_SOCKET_PATH: &str = "/tmp/sharo-daemon.sock";
const DEFAULT_STORE_PATH: &str = "/tmp/sharo-daemon-store.json";
const MAX_REQUEST_BYTES: usize = 1_048_576;

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
                summary,
            }) => DaemonResponse::SubmitTask(SubmitTaskOpResponse {
                task_id,
                task_state,
                summary,
            }),
            Err(message) => DaemonResponse::Error { message },
        },
        DaemonRequest::GetTask(payload) => match store.get_task(&payload.task_id) {
            Some(task) => DaemonResponse::GetTask(GetTaskResponse { task }),
            None => DaemonResponse::Error {
                message: format!("task_not_found task_id={}", payload.task_id),
            },
        },
        DaemonRequest::GetTrace(payload) => match store.get_trace(&payload.task_id) {
            Some(trace) => DaemonResponse::GetTrace(GetTraceResponse { trace }),
            None => DaemonResponse::Error {
                message: format!("trace_not_found task_id={}", payload.task_id),
            },
        },
        DaemonRequest::GetArtifacts(payload) => DaemonResponse::GetArtifacts(GetArtifactsResponse {
            artifacts: store.get_artifacts(&payload.task_id),
        }),
        DaemonRequest::ListPendingApprovals => {
            DaemonResponse::ListPendingApprovals(store.list_pending_approvals())
        }
        DaemonRequest::ResolveApproval(payload) => {
            match store.resolve_approval(&payload.approval_id, &payload.decision) {
                Ok(response) => DaemonResponse::ResolveApproval(response),
                Err(message) => DaemonResponse::Error { message },
            }
        }
    }
}

async fn handle_stream(stream: UnixStream, client: &impl RuntimeClient, store: &mut Store) {
    let (mut reader, mut writer) = stream.into_split();
    let mut bytes = Vec::new();

    loop {
        match reader.read_u8().await {
            Ok(b) => {
                if b == b'\n' {
                    break;
                }
                bytes.push(b);
                if bytes.len() > MAX_REQUEST_BYTES {
                    let _ = write_response(
                        &mut writer,
                        &DaemonResponse::Error {
                            message: format!(
                                "request_too_large max_bytes={} actual_bytes>{}",
                                MAX_REQUEST_BYTES, MAX_REQUEST_BYTES
                            ),
                        },
                    )
                    .await;
                    return;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
                break;
            }
            Err(error) => {
                let _ = write_response(
                    &mut writer,
                    &DaemonResponse::Error {
                        message: format!("read failure: {}", error),
                    },
                )
                .await;
                return;
            }
        }
    }

    if bytes.is_empty() {
        let _ = write_response(
            &mut writer,
            &DaemonResponse::Error {
                message: "empty request".to_string(),
            },
        )
        .await;
        return;
    }

    let line = match String::from_utf8(bytes) {
        Ok(line) => line,
        Err(error) => {
            let _ = write_response(
                &mut writer,
                &DaemonResponse::Error {
                    message: format!("invalid utf-8 request: {}", error),
                },
            )
            .await;
            return;
        }
    };

    let response = match serde_json::from_str::<DaemonRequest>(line.trim()) {
        Ok(request) => handle_request(request, client, store),
        Err(error) => DaemonResponse::Error {
            message: format!("invalid request: {error}"),
        },
    };
    let _ = write_response(&mut writer, &response).await;
}

async fn write_response<W>(writer: &mut W, response: &DaemonResponse) -> std::io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    let payload = serde_json::to_string(response).unwrap_or_else(|_| {
        serde_json::to_string(&DaemonResponse::Error {
            message: "serialization failure".to_string(),
        })
        .unwrap_or_else(|_| "{\"Error\":{\"message\":\"serialization failure\"}}".to_string())
    });
    writer.write_all(payload.as_bytes()).await?;
    writer.write_all(b"\n").await
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
            #[cfg(unix)]
            if let Err(error) = fs::set_permissions(&socket_path, fs::Permissions::from_mode(0o600)) {
                eprintln!("daemon_error=socket_permission_failed message={}", error);
                std::process::exit(1);
            }

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
