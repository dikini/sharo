use std::path::PathBuf;

use clap::{Parser, Subcommand};
use sharo_core::client::{RuntimeClient, StubClient};
use sharo_core::protocol::{DaemonRequest, DaemonResponse, TaskStatusRequest};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

const DEFAULT_SOCKET_PATH: &str = "/tmp/sharo-daemon.sock";

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
        #[arg(long)]
        serve_once: bool,
    },
}

fn handle_request(request: DaemonRequest, client: &impl RuntimeClient) -> DaemonResponse {
    match request {
        DaemonRequest::Submit(submit) => DaemonResponse::Submit(client.submit(&submit)),
        DaemonRequest::Status(status) => DaemonResponse::Status(client.status(&TaskStatusRequest {
            task_id: status.task_id,
        })),
    }
}

async fn handle_stream(stream: UnixStream, client: &impl RuntimeClient) {
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
                Ok(request) => handle_request(request, client),
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

                        handle_stream(stream, &client).await;

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
