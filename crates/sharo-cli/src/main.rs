use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use sharo_core::client::{RuntimeClient, StubClient};
use sharo_core::protocol::{
    CancelHazelSleepJobRequest, DaemonRequest, DaemonResponse, GetArtifactsRequest,
    EnqueueHazelSleepJobRequest, GetTaskRequest, GetTraceRequest, HazelConversationMessage,
    HazelRetrievalPreviewRequest, HazelSleepJobState, ListHazelCardsRequest,
    ListHazelProposalBatchesRequest, ListHazelSleepJobsRequest, PrePromptComposeHookInput,
    RegisterSessionRequest, ResolveApprovalRequest, SubmitHazelProposalBatchRequest,
    SubmitTaskOpRequest, SubmitTaskRequest, TaskStatusRequest,
    ValidateHazelProposalBatchRequest,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

const DEFAULT_SOCKET_PATH: &str = "/tmp/sharo-daemon.sock";

fn encode_field_value(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                encoded.push('%');
                encoded.push_str(&format!("{byte:02X}"));
            }
        }
    }
    encoded
}

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
    Approval {
        #[command(subcommand)]
        command: ApprovalCommand,
    },
    Hazel {
        #[command(subcommand)]
        command: HazelCommand,
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

#[derive(Debug, Subcommand)]
enum ApprovalCommand {
    List,
    Resolve {
        #[arg(long)]
        approval_id: String,
        #[arg(long)]
        decision: String,
    },
}

#[derive(Debug, Subcommand)]
enum HazelCommand {
    Status,
    Cards {
        #[arg(long, default_value_t = 8)]
        limit: u32,
    },
    Batches {
        #[arg(long, default_value_t = 8)]
        limit: u32,
    },
    Jobs {
        #[arg(long, default_value_t = 8)]
        limit: u32,
    },
    Preview {
        #[arg(long)]
        goal: String,
    },
    Submit {
        #[arg(long)]
        batch_id: String,
    },
    Validate {
        #[arg(long)]
        batch_id: String,
    },
    EnqueueJob {
        #[arg(long)]
        source_ref: String,
        #[arg(long)]
        idempotency_key: String,
        #[arg(long)]
        message: String,
    },
    CancelJob {
        #[arg(long)]
        job_id: String,
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

async fn send_ipc(
    socket_path: &PathBuf,
    request: &DaemonRequest,
) -> Result<DaemonResponse, String> {
    let stream = UnixStream::connect(socket_path).await.map_err(|error| {
        format!(
            "connect_failed path={} error={}",
            socket_path.display(),
            error
        )
    })?;
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
                        "task_id={} task_state={} current_step_summary={} blocking_reason={} coordination_summary={} result_preview={}",
                        response.task.task_id,
                        response.task.task_state,
                        response.task.current_step_summary,
                        response
                            .task
                            .blocking_reason
                            .unwrap_or_else(|| "none".to_string()),
                        response
                            .task
                            .coordination_summary
                            .unwrap_or_else(|| "none".to_string()),
                        response
                            .task
                            .result_preview
                            .map(|value| encode_field_value(&value))
                            .unwrap_or_else(|| "none".to_string())
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
                        "trace_id={} task_id={} session_id={} events={}",
                        response.trace.trace_id,
                        response.trace.task_id,
                        response.trace.session_id,
                        response.trace.events.len()
                    );
                    for event in response.trace.events {
                        println!(
                            "event_sequence={} event_kind={} details={}",
                            event.event_sequence, event.event_kind, event.details
                        );
                    }
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
                    for artifact in response.artifacts {
                        println!(
                            "artifact_id={} artifact_kind={} summary={} produced_by_step_id={} produced_by_trace_event_sequence={}",
                            artifact.artifact_id,
                            artifact.artifact_kind,
                            artifact.summary,
                            artifact.produced_by_step_id,
                            artifact.produced_by_trace_event_sequence
                        );
                    }
                    Ok(())
                }
                DaemonResponse::Error { message } => Err(format!("daemon_error={}", message)),
                other => Err(format!("unexpected_response={:?}", other)),
            }
        }
        Command::Approval {
            command: ApprovalCommand::List,
        } => {
            let request = DaemonRequest::ListPendingApprovals;
            match send_ipc(&cli.socket_path, &request).await? {
                DaemonResponse::ListPendingApprovals(response) => {
                    println!("pending_approvals={}", response.approvals.len());
                    for approval in response.approvals {
                        println!(
                            "approval_id={} task_id={} state={} reason={}",
                            approval.approval_id, approval.task_id, approval.state, approval.reason
                        );
                    }
                    Ok(())
                }
                DaemonResponse::Error { message } => Err(format!("daemon_error={}", message)),
                other => Err(format!("unexpected_response={:?}", other)),
            }
        }
        Command::Approval {
            command:
                ApprovalCommand::Resolve {
                    approval_id,
                    decision,
                },
        } => {
            let request = DaemonRequest::ResolveApproval(ResolveApprovalRequest {
                approval_id: approval_id.clone(),
                decision: decision.clone(),
            });
            match send_ipc(&cli.socket_path, &request).await? {
                DaemonResponse::ResolveApproval(response) => {
                    println!(
                        "approval_id={} task_id={} state={}",
                        response.approval_id, response.task_id, response.state
                    );
                    Ok(())
                }
                DaemonResponse::Error { message } => Err(format!("daemon_error={}", message)),
                other => Err(format!("unexpected_response={:?}", other)),
            }
        }
        Command::Hazel {
            command: HazelCommand::Status,
        } => match send_ipc(&cli.socket_path, &DaemonRequest::GetHazelStatus).await? {
            DaemonResponse::GetHazelStatus(response) => {
                println!(
                    "available={} cards={} proposal_batches={} sleep_jobs={}",
                    response.status.available,
                    response.status.card_count,
                    response.status.proposal_batch_count,
                    response.status.sleep_job_count
                );
                Ok(())
            }
            DaemonResponse::Error { message } => Err(format!("daemon_error={}", message)),
            other => Err(format!("unexpected_response={:?}", other)),
        },
        Command::Hazel {
            command: HazelCommand::Cards { limit },
        } => match send_ipc(
            &cli.socket_path,
            &DaemonRequest::ListHazelCards(ListHazelCardsRequest { limit: Some(*limit) }),
        )
        .await?
        {
            DaemonResponse::ListHazelCards(response) => {
                println!("hazel_cards={}", response.cards.len());
                for card in response.cards {
                    println!(
                        "card_id={} subject={} provenance={}",
                        card.card_id,
                        encode_field_value(&card.subject),
                        card.provenance.len()
                    );
                }
                Ok(())
            }
            DaemonResponse::Error { message } => Err(format!("daemon_error={}", message)),
            other => Err(format!("unexpected_response={:?}", other)),
        },
        Command::Hazel {
            command: HazelCommand::Batches { limit },
        } => match send_ipc(
            &cli.socket_path,
            &DaemonRequest::ListHazelProposalBatches(ListHazelProposalBatchesRequest {
                limit: Some(*limit),
            }),
        )
        .await?
        {
            DaemonResponse::ListHazelProposalBatches(response) => {
                println!("hazel_batches={}", response.batches.len());
                for batch in response.batches {
                    println!(
                        "batch_id={} source_ref={} proposal_count={}",
                        batch.batch_id,
                        encode_field_value(&batch.source_ref),
                        batch.proposal_count
                    );
                }
                Ok(())
            }
            DaemonResponse::Error { message } => Err(format!("daemon_error={}", message)),
            other => Err(format!("unexpected_response={:?}", other)),
        },
        Command::Hazel {
            command: HazelCommand::Jobs { limit },
        } => match send_ipc(
            &cli.socket_path,
            &DaemonRequest::ListHazelSleepJobs(ListHazelSleepJobsRequest { limit: Some(*limit) }),
        )
        .await?
        {
            DaemonResponse::ListHazelSleepJobs(response) => {
                println!("hazel_jobs={}", response.jobs.len());
                for job in response.jobs {
                    println!(
                        "job_id={} state={} batches={}",
                        job.job_id,
                        hazel_sleep_job_state_label(job.state),
                        job.proposal_batch_ids.len()
                    );
                }
                Ok(())
            }
            DaemonResponse::Error { message } => Err(format!("daemon_error={}", message)),
            other => Err(format!("unexpected_response={:?}", other)),
        },
        Command::Hazel {
            command: HazelCommand::Preview { goal },
        } => match send_ipc(
            &cli.socket_path,
            &DaemonRequest::HazelPreview(HazelRetrievalPreviewRequest {
                input: PrePromptComposeHookInput {
                    session_id: "operator".to_string(),
                    task_id: "hazel-preview".to_string(),
                    goal: goal.clone(),
                    runtime: "operator".to_string(),
                    top_k: Some(3),
                    token_budget: Some(128),
                    relevance_threshold: Some(0.0),
                    policy_ids: vec!["hunch.v1".to_string()],
                    card_policy_hints: Vec::new(),
                },
            }),
        )
        .await?
        {
            DaemonResponse::HazelPreview(response) => {
                println!(
                    "preview_id={} cards={}",
                    response.preview_id,
                    response.payload.cards.len()
                );
                Ok(())
            }
            DaemonResponse::Error { message } => Err(format!("daemon_error={}", message)),
            other => Err(format!("unexpected_response={:?}", other)),
        },
        Command::Hazel {
            command: HazelCommand::Submit { batch_id },
        } => match send_ipc(
            &cli.socket_path,
            &DaemonRequest::SubmitHazelProposalBatch(SubmitHazelProposalBatchRequest {
                batch_id: batch_id.clone(),
                strict_policy_ids: vec!["hunch.v1".to_string()],
            }),
        )
        .await?
        {
            DaemonResponse::SubmitHazelProposalBatch(response) => {
                println!(
                    "submission_id={} batch_id={} state={}",
                    response.submission_id, response.batch_id, response.state
                );
                Ok(())
            }
            DaemonResponse::Error { message } => Err(format!("daemon_error={}", message)),
            other => Err(format!("unexpected_response={:?}", other)),
        },
        Command::Hazel {
            command: HazelCommand::Validate { batch_id },
        } => match send_ipc(
            &cli.socket_path,
            &DaemonRequest::ValidateHazelProposalBatch(ValidateHazelProposalBatchRequest {
                batch_id: batch_id.clone(),
                strict_policy_ids: vec!["hunch.v1".to_string()],
            }),
        )
        .await?
        {
            DaemonResponse::ValidateHazelProposalBatch(response) => {
                println!(
                    "validation_id={} batch_id={} accepted={} summary={}",
                    response.validation_id,
                    response.batch_id,
                    response.accepted,
                    encode_field_value(&response.summary)
                );
                Ok(())
            }
            DaemonResponse::Error { message } => Err(format!("daemon_error={}", message)),
            other => Err(format!("unexpected_response={:?}", other)),
        },
        Command::Hazel {
            command:
                HazelCommand::EnqueueJob {
                    source_ref,
                    idempotency_key,
                    message,
                },
        } => match send_ipc(
            &cli.socket_path,
            &DaemonRequest::EnqueueHazelSleepJob(EnqueueHazelSleepJobRequest {
                job_id: None,
                source_ref: source_ref.clone(),
                idempotency_key: idempotency_key.clone(),
                messages: vec![parse_hazel_message(message)?],
                max_batches: 8,
                max_proposals_per_batch: 64,
            }),
        )
        .await?
        {
            DaemonResponse::EnqueueHazelSleepJob(response) => {
                println!(
                    "job_id={} state={} proposal_batches={}",
                    response.job.job_id,
                    hazel_sleep_job_state_label(response.job.state),
                    response.proposal_batch_ids.len()
                );
                Ok(())
            }
            DaemonResponse::Error { message } => Err(format!("daemon_error={}", message)),
            other => Err(format!("unexpected_response={:?}", other)),
        },
        Command::Hazel {
            command: HazelCommand::CancelJob { job_id },
        } => match send_ipc(
            &cli.socket_path,
            &DaemonRequest::CancelHazelSleepJob(CancelHazelSleepJobRequest {
                job_id: job_id.clone(),
            }),
        )
        .await?
        {
            DaemonResponse::CancelHazelSleepJob(response) => {
                println!(
                    "job_id={} state={}",
                    response.job.job_id,
                    hazel_sleep_job_state_label(response.job.state)
                );
                Ok(())
            }
            DaemonResponse::Error { message } => Err(format!("daemon_error={}", message)),
            other => Err(format!("unexpected_response={:?}", other)),
        },
    }
}

fn parse_hazel_message(input: &str) -> Result<HazelConversationMessage, String> {
    let (role, content) = input
        .split_once(':')
        .ok_or_else(|| "hazel_message_invalid expected=role: content".to_string())?;
    let role = role.trim();
    let content = content.trim();
    if role.is_empty() || content.is_empty() {
        return Err("hazel_message_invalid expected=role: content".to_string());
    }
    Ok(HazelConversationMessage {
        role: role.to_string(),
        content: content.to_string(),
    })
}

fn hazel_sleep_job_state_label(state: HazelSleepJobState) -> &'static str {
    match state {
        HazelSleepJobState::Pending => "pending",
        HazelSleepJobState::Completed => "completed",
        HazelSleepJobState::Failed => "failed",
        HazelSleepJobState::Canceled => "canceled",
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

#[cfg(test)]
mod tests {
    use super::encode_field_value;

    #[test]
    fn encode_field_value_preserves_single_token_output() {
        assert_eq!(
            encode_field_value("hello world\nnext=line"),
            "hello%20world%0Anext%3Dline"
        );
    }

    #[test]
    fn encode_field_value_keeps_safe_ascii_readable() {
        assert_eq!(
            encode_field_value("deterministic-response"),
            "deterministic-response"
        );
    }
}
