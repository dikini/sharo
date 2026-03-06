use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
#[cfg(unix)]
use std::{fs, os::unix::fs::PermissionsExt};

use clap::{Parser, Subcommand};
use sharo_core::client::{RuntimeClient, StubClient};
use sharo_core::kernel::KernelApprovalInput;
use sharo_core::protocol::{
    DaemonRequest, DaemonResponse, GetArtifactsResponse, GetTaskResponse, GetTraceResponse,
    RegisterSessionResponse, SubmitTaskOpResponse, TaskStatusRequest,
};
use sharo_core::reasoning::ReasoningError;
use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};

mod store;
mod config;
mod connector_pool;
mod kernel;
use config::{default_daemon_config_path, load_daemon_config};
use kernel::{DaemonKernel, DaemonKernelRuntime, KernelRuntimeConfig};
use store::{Store, SubmitPreparationOutcome, SubmitReplay};

const DEFAULT_SOCKET_PATH: &str = "/tmp/sharo-daemon.sock";
const DEFAULT_STORE_PATH: &str = "/tmp/sharo-daemon-store.json";
const MAX_REQUEST_BYTES: usize = 1_048_576;

struct AppState {
    client: StubClient,
    store: Mutex<Store>,
    daemon_kernel: DaemonKernel,
}

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
        config_path: Option<PathBuf>,
        #[arg(long)]
        serve_once: bool,
    },
}

fn handle_request(
    request: DaemonRequest,
    state: &AppState,
) -> DaemonResponse {
    match request {
        DaemonRequest::Submit(submit) => DaemonResponse::Submit(state.client.submit(&submit)),
        DaemonRequest::Status(status) => DaemonResponse::Status(state.client.status(&TaskStatusRequest {
            task_id: status.task_id,
        })),
        DaemonRequest::RegisterSession(payload) => match lock_unpoisoned(&state.store)
            .register_session(&payload.session_label)
        {
            Ok(session_id) => DaemonResponse::RegisterSession(RegisterSessionResponse { session_id }),
            Err(message) => DaemonResponse::Error { message },
        },
        DaemonRequest::SubmitTask(payload) => match handle_submit_task(state, payload) {
            Ok(response) => DaemonResponse::SubmitTask(response),
            Err(message) => DaemonResponse::Error { message },
        },
        DaemonRequest::GetTask(payload) => match lock_unpoisoned(&state.store).get_task(&payload.task_id) {
            Some(task) => DaemonResponse::GetTask(GetTaskResponse { task }),
            None => DaemonResponse::Error {
                message: format!("task_not_found task_id={}", payload.task_id),
            },
        },
        DaemonRequest::GetTrace(payload) => match lock_unpoisoned(&state.store).get_trace(&payload.task_id) {
            Some(trace) => DaemonResponse::GetTrace(GetTraceResponse { trace }),
            None => DaemonResponse::Error {
                message: format!("trace_not_found task_id={}", payload.task_id),
            },
        },
        DaemonRequest::GetArtifacts(payload) => DaemonResponse::GetArtifacts(GetArtifactsResponse {
            artifacts: lock_unpoisoned(&state.store).get_artifacts(&payload.task_id),
        }),
        DaemonRequest::ListPendingApprovals => {
            DaemonResponse::ListPendingApprovals(lock_unpoisoned(&state.store).list_pending_approvals())
        }
        DaemonRequest::ResolveApproval(payload) => {
            let mut store = lock_unpoisoned(&state.store);
            let mut kernel = DaemonKernelRuntime::new(&mut store);
            match kernel.resolve_approval(KernelApprovalInput {
                approval_id: payload.approval_id,
                decision: payload.decision,
            }) {
                Ok(response) => DaemonResponse::ResolveApproval(response.response),
                Err(message) => DaemonResponse::Error { message },
            }
        }
    }
}

fn handle_submit_task(
    state: &AppState,
    payload: sharo_core::protocol::SubmitTaskOpRequest,
) -> Result<SubmitTaskOpResponse, String> {
    let preparation = {
        let mut store = lock_unpoisoned(&state.store);
        store.prepare_submit(&payload)?
    };

    let preparation = match preparation {
        SubmitPreparationOutcome::Replay(SubmitReplay::Task(response)) => return Ok(response),
        SubmitPreparationOutcome::Replay(SubmitReplay::Error(message)) => return Err(message),
        SubmitPreparationOutcome::Ready(preparation) => preparation,
    };

    let reasoning = state.daemon_kernel.reason_submit(&preparation, &payload);
    let mut store = lock_unpoisoned(&state.store);
    match reasoning {
        Ok(reasoning) => store.submit_task_with_route(
            &preparation,
            payload,
            &reasoning.route_decision_details,
            &reasoning.model_output_text,
            &reasoning.fit_loop_records,
        ),
        Err(ReasoningError::FitLoopFailure { message, records }) => {
            store.submit_failed_task(
                &preparation,
                payload,
                &message,
                &records,
            )
        }
        Err(ReasoningError::ConnectorFailure { message })
        | Err(ReasoningError::ResolveFailure { message }) => {
            store.record_submission_failure(
                &preparation.session_id_hint,
                payload.idempotency_key.as_deref(),
                &message,
            )?;
            Err(message)
        }
    }
}

fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

async fn handle_stream(
    stream: UnixStream,
    state: Arc<AppState>,
) {
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
        Ok(request) => match tokio::task::spawn_blocking({
            let state = Arc::clone(&state);
            move || handle_request(request, &state)
        })
        .await
        {
            Ok(response) => response,
            Err(error) => DaemonResponse::Error {
                message: format!("request_task_join_failed error={error}"),
            },
        },
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
            config_path,
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

            let store = match Store::open(&store_path) {
                Ok(store) => store,
                Err(message) => {
                    eprintln!("daemon_error=store_open_failed message={}", message);
                    std::process::exit(1);
                }
            };
            let resolved_config_path = config_path.or_else(default_daemon_config_path);
            let daemon_config = match load_daemon_config(resolved_config_path.as_deref()) {
                Ok(cfg) => cfg,
                Err(message) => {
                    eprintln!("daemon_error=config_load_failed message={}", message);
                    std::process::exit(1);
                }
            };
            let kernel_config = match KernelRuntimeConfig::from_daemon_config(&daemon_config) {
                Ok(cfg) => cfg,
                Err(message) => {
                    eprintln!("daemon_error=config_invalid message={}", message);
                    std::process::exit(1);
                }
            };
            let state = Arc::new(AppState {
                client: StubClient,
                store: Mutex::new(store),
                daemon_kernel: DaemonKernel::new(&kernel_config),
            });

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

                        if serve_once {
                            handle_stream(stream, Arc::clone(&state)).await;
                            break;
                        }

                        tokio::spawn(handle_stream(stream, Arc::clone(&state)));

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
