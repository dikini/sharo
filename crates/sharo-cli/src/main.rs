use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use sharo_core::client::{RuntimeClient, StubClient};
use sharo_core::protocol::{
    DaemonRequest, DaemonResponse, GetArtifactsRequest, GetTaskRequest, GetTraceRequest,
    RegisterSessionRequest, SubmitTaskOpRequest, SubmitTaskRequest, TaskStatusRequest,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

const DEFAULT_SOCKET_PATH: &str = "/tmp/sharo-daemon.sock";

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Transport {
    Ipc,
    Stub,
}

#[derive(Debug, Parser)]
#[command(name = "sharo")]
#[command(about = "Sharo CLI")]
struct Cli {
    #[arg(long, value_enum, default_value_t = Transport::Ipc)]
    transport: Transport,
    #[arg(long, default_value = DEFAULT_SOCKET_PATH)]
    socket_path: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Submit {
        #[arg(long)]
        goal: String,
        #[arg(long)]
        session_id: Option<String>,
    },
    Status {
        #[arg(long)]
        task_id: String,
    },
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },
    Task {
        #[command(subcommand)]
        command: TaskCommand,
    },
    Trace {
        #[command(subcommand)]
        command: TraceCommand,
    },
    Artifacts {
        #[command(subcommand)]
        command: ArtifactsCommand,
    },
}

#[derive(Debug, Subcommand)]
enum SessionCommand {
    Open {
        #[arg(long)]
        label: String,
    },
}

#[derive(Debug, Subcommand)]
enum TaskCommand {
    Submit {
        #[arg(long)]
        goal: String,
        #[arg(long)]
        session_id: Option<String>,
    },
    Get {
        #[arg(long)]
        task_id: String,
    },
}

#[derive(Debug, Subcommand)]
enum TraceCommand {
    Get {
        #[arg(long)]
        task_id: String,
    },
}

#[derive(Debug, Subcommand)]
enum ArtifactsCommand {
    List {
        #[arg(long)]
        task_id: String,
    },
}

fn run_stub(client: &impl RuntimeClient, cli: &Cli) {
    match &cli.command {
        Command::Submit { goal, session_id } => {
            let response = client.submit(&SubmitTaskRequest {
                session_id: session_id.clone(),
                goal: goal.clone(),
            });
            println!("task_id={} state={:?}", response.task_id, response.state);
        }
        Command::Status { task_id } => {
            let response = client.status(&TaskStatusRequest {
                task_id: task_id.clone(),
            });
            println!(
                "task_id={} state={:?} summary={}",
                response.task_id, response.state, response.summary
            );
        }
        _ => {
            eprintln!("sharo_cli_error=stub_mode_only_supports_submit_status");
            std::process::exit(1);
        }
    }
}

async fn send_ipc(socket_path: &PathBuf, request: &DaemonRequest) -> Result<DaemonResponse, String> {
    let stream = UnixStream::connect(socket_path)
        .await
        .map_err(|error| format!("connect_failed path={} error={}", socket_path.display(), error))?;
    let (reader, mut writer) = stream.into_split();

    let payload = serde_json::to_string(request)
        .map_err(|error| format!("request_serialize_failed error={}", error))?;
    writer
        .write_all(payload.as_bytes())
        .await
        .map_err(|error| format!("request_write_failed error={}", error))?;
    writer
        .write_all(b"\n")
        .await
        .map_err(|error| format!("request_write_failed error={}", error))?;

    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .await
        .map_err(|error| format!("response_read_failed error={}", error))?;

    if line.trim().is_empty() {
        return Err("empty_response".to_string());
    }

    serde_json::from_str::<DaemonResponse>(line.trim())
        .map_err(|error| format!("response_parse_failed error={}", error))
}

async fn run_ipc(cli: &Cli) -> Result<(), String> {
    match &cli.command {
        Command::Submit { goal, session_id } => {
            let request = DaemonRequest::Submit(SubmitTaskRequest {
                session_id: session_id.clone(),
                goal: goal.clone(),
            });
            match send_ipc(&cli.socket_path, &request).await? {
                DaemonResponse::Submit(response) => {
                    println!("task_id={} state={:?}", response.task_id, response.state);
                    Ok(())
                }
                DaemonResponse::Error { message } => Err(format!("daemon_error={}", message)),
                other => Err(format!("unexpected_response={:?}", other)),
            }
        }
        Command::Status { task_id } => {
            let request = DaemonRequest::Status(TaskStatusRequest {
                task_id: task_id.clone(),
            });
            match send_ipc(&cli.socket_path, &request).await? {
                DaemonResponse::Status(response) => {
                    println!(
                        "task_id={} state={:?} summary={}",
                        response.task_id, response.state, response.summary
                    );
                    Ok(())
                }
                DaemonResponse::Error { message } => Err(format!("daemon_error={}", message)),
                other => Err(format!("unexpected_response={:?}", other)),
            }
        }
        Command::Session {
            command: SessionCommand::Open { label },
        } => {
            let request = DaemonRequest::RegisterSession(RegisterSessionRequest {
                session_label: label.clone(),
            });
            match send_ipc(&cli.socket_path, &request).await? {
                DaemonResponse::RegisterSession(response) => {
                    println!("session_id={}", response.session_id);
                    Ok(())
                }
                DaemonResponse::Error { message } => Err(format!("daemon_error={}", message)),
                other => Err(format!("unexpected_response={:?}", other)),
            }
        }
        Command::Task {
            command: TaskCommand::Submit { goal, session_id },
        } => {
            let request = DaemonRequest::SubmitTask(SubmitTaskOpRequest {
                session_id: session_id.clone(),
                goal: goal.clone(),
                idempotency_key: None,
            });
            match send_ipc(&cli.socket_path, &request).await? {
                DaemonResponse::SubmitTask(response) => {
                    println!(
                        "task_id={} task_state={} summary={}",
                        response.task_id, response.task_state, response.summary
                    );
                    Ok(())
                }
                DaemonResponse::Error { message } => Err(format!("daemon_error={}", message)),
                other => Err(format!("unexpected_response={:?}", other)),
            }
        }
        Command::Task {
            command: TaskCommand::Get { task_id },
        } => {
            let request = DaemonRequest::GetTask(GetTaskRequest {
                task_id: task_id.clone(),
            });
            match send_ipc(&cli.socket_path, &request).await? {
                DaemonResponse::GetTask(response) => {
                    println!(
                        "task_id={} task_state={} current_step_summary={}",
                        response.task.task_id,
                        response.task.task_state,
                        response.task.current_step_summary
                    );
                    Ok(())
                }
                DaemonResponse::Error { message } => Err(format!("daemon_error={}", message)),
                other => Err(format!("unexpected_response={:?}", other)),
            }
        }
        Command::Trace {
            command: TraceCommand::Get { task_id },
        } => {
            let request = DaemonRequest::GetTrace(GetTraceRequest {
                task_id: task_id.clone(),
            });
            match send_ipc(&cli.socket_path, &request).await? {
                DaemonResponse::GetTrace(response) => {
                    println!(
                        "trace_id={} task_id={} events={}",
                        response.trace.trace_id,
                        response.trace.task_id,
                        response.trace.events.len()
                    );
                    Ok(())
                }
                DaemonResponse::Error { message } => Err(format!("daemon_error={}", message)),
                other => Err(format!("unexpected_response={:?}", other)),
            }
        }
        Command::Artifacts {
            command: ArtifactsCommand::List { task_id },
        } => {
            let request = DaemonRequest::GetArtifacts(GetArtifactsRequest {
                task_id: task_id.clone(),
            });
            match send_ipc(&cli.socket_path, &request).await? {
                DaemonResponse::GetArtifacts(response) => {
                    println!("task_id={} artifacts={}", task_id, response.artifacts.len());
                    Ok(())
                }
                DaemonResponse::Error { message } => Err(format!("daemon_error={}", message)),
                other => Err(format!("unexpected_response={:?}", other)),
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.transport {
        Transport::Stub => {
            run_stub(&StubClient, &cli);
            Ok(())
        }
        Transport::Ipc => run_ipc(&cli).await,
    };

    if let Err(error) = result {
        eprintln!("sharo_cli_error={}", error);
        std::process::exit(1);
    }
}
