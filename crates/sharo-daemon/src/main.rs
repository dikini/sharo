use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
#[cfg(unix)]
use std::{fs, os::unix::fs::PermissionsExt};

use clap::{Parser, Subcommand};
use sharo_core::client::{RuntimeClient, StubClient};
use sharo_core::kernel::KernelApprovalInput;
use sharo_core::protocol::{
    CancelHazelSleepJobResponse, DaemonRequest, DaemonResponse, EnqueueHazelSleepJobResponse,
    GetArtifactsResponse, GetHazelCardResponse,
    GetHazelProposalBatchResponse, GetHazelSleepJobResponse, GetHazelStatusResponse,
    GetRuntimeStatusResponse, GetSessionTasksResponse, GetSessionViewResponse, GetTaskResponse,
    GetTraceResponse, HazelRetrievalPreviewResponse, ListHazelCardsResponse,
    ListHazelProposalBatchesResponse, ListHazelSleepJobsResponse, ListMcpServersResponse,
    RegisterSessionResponse, SubmitHazelProposalBatchResponse, SubmitTaskOpResponse,
    TaskStatusRequest, UpdateMcpServerStateResponse, ValidateHazelProposalBatchResponse,
};
use sharo_core::reasoning::ReasoningError;
use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::task::JoinSet;

mod config;
mod connector_pool;
mod control_plane;
mod hazel_control_plane;
mod kernel;
mod mcp_registry;
mod skills;
mod store;
use config::{default_daemon_config_path, load_daemon_config};
use kernel::{DaemonKernel, DaemonKernelRuntime, KernelRuntimeConfig};
use mcp_registry::McpRegistry;
use skills::load_skill_catalog;
use store::{Store, SubmitPreparationOutcome, SubmitReplay};

const DEFAULT_SOCKET_PATH: &str = "/tmp/sharo-daemon.sock";
const DEFAULT_STORE_PATH: &str = "/tmp/sharo-daemon-store.json";
const MAX_REQUEST_BYTES: usize = 1_048_576;

struct AppState {
    client: StubClient,
    store: Mutex<Store>,
    daemon_kernel: DaemonKernel,
    skills_catalog: skills::SkillCatalog,
    mcp_registry: McpRegistry,
    model_config: config::ModelRuntimeConfig,
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

fn handle_request(request: DaemonRequest, state: &AppState) -> DaemonResponse {
    match request {
        DaemonRequest::Submit(submit) => DaemonResponse::Submit(state.client.submit(&submit)),
        DaemonRequest::Status(status) => {
            DaemonResponse::Status(state.client.status(&TaskStatusRequest {
                task_id: status.task_id,
            }))
        }
        DaemonRequest::RegisterSession(payload) => {
            match lock_unpoisoned(&state.store).register_session(&payload.session_label) {
                Ok(session_id) => {
                    DaemonResponse::RegisterSession(RegisterSessionResponse { session_id })
                }
                Err(message) => DaemonResponse::Error { message },
            }
        }
        DaemonRequest::ListSessions => DaemonResponse::ListSessions(control_plane::list_sessions(
            &lock_unpoisoned(&state.store),
        )),
        DaemonRequest::SubmitTask(payload) => match handle_submit_task(state, payload) {
            Ok(response) => DaemonResponse::SubmitTask(response),
            Err(message) => DaemonResponse::Error { message },
        },
        DaemonRequest::GetTask(payload) => {
            match lock_unpoisoned(&state.store).get_task(&payload.task_id) {
                Some(task) => DaemonResponse::GetTask(GetTaskResponse { task }),
                None => DaemonResponse::Error {
                    message: format!("task_not_found task_id={}", payload.task_id),
                },
            }
        }
        DaemonRequest::GetSessionTasks(payload) => match control_plane::get_session_tasks(
            &lock_unpoisoned(&state.store),
            &payload.session_id,
            payload.task_limit,
        ) {
            Some(GetSessionTasksResponse { tasks }) => {
                DaemonResponse::GetSessionTasks(GetSessionTasksResponse { tasks })
            }
            None => DaemonResponse::Error {
                message: format!("session_not_found session_id={}", payload.session_id),
            },
        },
        DaemonRequest::GetSessionView(payload) => match control_plane::get_session_view(
            &lock_unpoisoned(&state.store),
            &payload.session_id,
            payload.task_limit,
        ) {
            Some(GetSessionViewResponse { session }) => {
                DaemonResponse::GetSessionView(GetSessionViewResponse { session })
            }
            None => DaemonResponse::Error {
                message: format!("session_not_found session_id={}", payload.session_id),
            },
        },
        DaemonRequest::ListSkills(payload) => {
            let active_skills = if let Some(session_id) = payload.session_id.as_deref() {
                match lock_unpoisoned(&state.store).list_session_active_skills(session_id) {
                    Some(active_skills) => active_skills,
                    None => {
                        return DaemonResponse::Error {
                            message: format!("session_not_found session_id={session_id}"),
                        };
                    }
                }
            } else {
                Vec::new()
            };
            DaemonResponse::ListSkills(sharo_core::protocol::ListSkillsResponse {
                skills: state.skills_catalog.list(&active_skills),
            })
        }
        DaemonRequest::GetSkill(payload) => match state.skills_catalog.get(&payload.skill_id) {
            Ok(Some(skill)) => {
                DaemonResponse::GetSkill(sharo_core::protocol::GetSkillResponse { skill })
            }
            Ok(None) => DaemonResponse::Error {
                message: format!("skill_not_found skill_id={}", payload.skill_id),
            },
            Err(message) => DaemonResponse::Error { message },
        },
        DaemonRequest::SetSessionSkills(payload) => {
            let active_skill_ids = match state
                .skills_catalog
                .validate_skill_ids(&payload.active_skill_ids)
            {
                Ok(skill_ids) => skill_ids,
                Err(message) => return DaemonResponse::Error { message },
            };
            match lock_unpoisoned(&state.store)
                .set_session_active_skills(&payload.session_id, active_skill_ids)
            {
                Ok(active_skill_ids) => DaemonResponse::SetSessionSkills(
                    sharo_core::protocol::SetSessionSkillsResponse {
                        session_id: payload.session_id,
                        active_skill_ids,
                    },
                ),
                Err(message) => DaemonResponse::Error { message },
            }
        }
        DaemonRequest::ListMcpServers => {
            let store = lock_unpoisoned(&state.store);
            let servers = state.mcp_registry.list_servers(&mcp_overrides(&store));
            DaemonResponse::ListMcpServers(ListMcpServersResponse { servers })
        }
        DaemonRequest::UpdateMcpServerState(payload) => {
            if !state.mcp_registry.contains_server(&payload.server_id) {
                return DaemonResponse::Error {
                    message: format!("mcp_server_not_found server_id={}", payload.server_id),
                };
            }
            let mut store = lock_unpoisoned(&state.store);
            match store.set_mcp_enabled_override(&payload.server_id, payload.enabled) {
                Ok(enabled) => {
                    let server = state
                        .mcp_registry
                        .get_server(&payload.server_id, Some(enabled))
                        .expect("validated mcp server must exist");
                    DaemonResponse::UpdateMcpServerState(UpdateMcpServerStateResponse { server })
                }
                Err(message) => DaemonResponse::Error { message },
            }
        }
        DaemonRequest::GetRuntimeStatus => {
            let store = lock_unpoisoned(&state.store);
            let status = state
                .mcp_registry
                .runtime_status(&mcp_overrides(&store), &state.model_config);
            DaemonResponse::GetRuntimeStatus(GetRuntimeStatusResponse { status })
        }
        DaemonRequest::GetHazelStatus => {
            DaemonResponse::GetHazelStatus(GetHazelStatusResponse {
                status: hazel_control_plane::get_hazel_status(&lock_unpoisoned(&state.store)).status,
            })
        }
        DaemonRequest::ListHazelCards(payload) => {
            DaemonResponse::ListHazelCards(ListHazelCardsResponse {
                cards: hazel_control_plane::list_hazel_cards(payload.limit).cards,
            })
        }
        DaemonRequest::GetHazelCard(payload) => {
            match hazel_control_plane::get_hazel_card(&payload.card_id) {
                Some(card) => DaemonResponse::GetHazelCard(GetHazelCardResponse { card }),
                None => DaemonResponse::Error {
                    message: format!("hazel_card_not_found card_id={}", payload.card_id),
                },
            }
        }
        DaemonRequest::ListHazelProposalBatches(payload) => {
            DaemonResponse::ListHazelProposalBatches(ListHazelProposalBatchesResponse {
                batches: hazel_control_plane::list_hazel_proposal_batches(
                    &lock_unpoisoned(&state.store),
                    payload.limit,
                )
                .batches,
            })
        }
        DaemonRequest::GetHazelProposalBatch(payload) => {
            match hazel_control_plane::get_hazel_proposal_batch(
                &lock_unpoisoned(&state.store),
                &payload.batch_id,
            ) {
                Some(batch) => {
                    DaemonResponse::GetHazelProposalBatch(GetHazelProposalBatchResponse { batch })
                }
                None => DaemonResponse::Error {
                    message: format!("hazel_proposal_batch_not_found batch_id={}", payload.batch_id),
                },
            }
        }
        DaemonRequest::ListHazelSleepJobs(payload) => {
            DaemonResponse::ListHazelSleepJobs(ListHazelSleepJobsResponse {
                jobs: hazel_control_plane::list_hazel_sleep_jobs(
                    &lock_unpoisoned(&state.store),
                    payload.limit,
                )
                .jobs,
            })
        }
        DaemonRequest::GetHazelSleepJob(payload) => {
            match hazel_control_plane::get_hazel_sleep_job(
                &lock_unpoisoned(&state.store),
                &payload.job_id,
            ) {
                Some(job) => DaemonResponse::GetHazelSleepJob(GetHazelSleepJobResponse { job }),
                None => DaemonResponse::Error {
                    message: format!("hazel_sleep_job_not_found job_id={}", payload.job_id),
                },
            }
        }
        DaemonRequest::HazelPreview(payload) => {
            match hazel_control_plane::preview_hazel_retrieval(
                &mut lock_unpoisoned(&state.store),
                payload,
            ) {
                Ok(response) => {
                    DaemonResponse::HazelPreview(HazelRetrievalPreviewResponse { ..response })
                }
                Err(message) => DaemonResponse::Error { message },
            }
        }
        DaemonRequest::ValidateHazelProposalBatch(payload) => {
            match hazel_control_plane::validate_hazel_proposal_batch_action(
                &mut lock_unpoisoned(&state.store),
                payload,
            ) {
                Ok(response) => DaemonResponse::ValidateHazelProposalBatch(
                    ValidateHazelProposalBatchResponse { ..response },
                ),
                Err(message) => DaemonResponse::Error { message },
            }
        }
        DaemonRequest::SubmitHazelProposalBatch(payload) => {
            match hazel_control_plane::submit_hazel_proposal_batch_action(
                &mut lock_unpoisoned(&state.store),
                payload,
            ) {
                Ok(response) => DaemonResponse::SubmitHazelProposalBatch(
                    SubmitHazelProposalBatchResponse { ..response },
                ),
                Err(message) => DaemonResponse::Error { message },
            }
        }
        DaemonRequest::EnqueueHazelSleepJob(payload) => {
            match hazel_control_plane::enqueue_hazel_sleep_job_action(
                &mut lock_unpoisoned(&state.store),
                payload,
            ) {
                Ok(response) => DaemonResponse::EnqueueHazelSleepJob(
                    EnqueueHazelSleepJobResponse { ..response },
                ),
                Err(message) => DaemonResponse::Error { message },
            }
        }
        DaemonRequest::CancelHazelSleepJob(payload) => {
            match hazel_control_plane::cancel_hazel_sleep_job_action(
                &mut lock_unpoisoned(&state.store),
                &payload.job_id,
            ) {
                Ok(response) => DaemonResponse::CancelHazelSleepJob(
                    CancelHazelSleepJobResponse { ..response },
                ),
                Err(message) => DaemonResponse::Error { message },
            }
        }
        DaemonRequest::GetTrace(payload) => {
            match lock_unpoisoned(&state.store).get_trace(&payload.task_id) {
                Some(trace) => DaemonResponse::GetTrace(GetTraceResponse { trace }),
                None => DaemonResponse::Error {
                    message: format!("trace_not_found task_id={}", payload.task_id),
                },
            }
        }
        DaemonRequest::GetArtifacts(payload) => {
            DaemonResponse::GetArtifacts(GetArtifactsResponse {
                artifacts: lock_unpoisoned(&state.store).get_artifacts(&payload.task_id),
            })
        }
        DaemonRequest::ListPendingApprovals => DaemonResponse::ListPendingApprovals(
            lock_unpoisoned(&state.store).list_pending_approvals(),
        ),
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

fn mcp_overrides(store: &Store) -> BTreeMap<String, bool> {
    store.list_mcp_enabled_overrides()
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
    let idempotency_key = payload.idempotency_key.clone();
    match reasoning {
        Ok(reasoning) => match store.submit_task_with_route(
            &preparation,
            payload,
            &reasoning.route_decision_details,
            &reasoning.model_output_text,
            &reasoning.fit_loop_records,
        ) {
            Ok(response) => Ok(response),
            Err(message) => {
                store.release_inflight_idempotency_retry_lock(
                    &preparation.session_id_hint,
                    idempotency_key.as_deref(),
                    &preparation.task_id_hint,
                );
                Err(message)
            }
        },
        Err(ReasoningError::FitLoopFailure { message, records }) => {
            match store.submit_failed_task(&preparation, payload, &message, &records) {
                Ok(response) => Ok(response),
                Err(store_error) => {
                    store.release_inflight_idempotency_retry_lock(
                        &preparation.session_id_hint,
                        idempotency_key.as_deref(),
                        &preparation.task_id_hint,
                    );
                    Err(store_error)
                }
            }
        }
        Err(ReasoningError::ConnectorFailure { message })
        | Err(ReasoningError::ResolveFailure { message }) => {
            if let Err(store_error) = store.record_submission_failure(
                &preparation.session_id_hint,
                payload.idempotency_key.as_deref(),
                &message,
            ) {
                store.release_inflight_idempotency_retry_lock(
                    &preparation.session_id_hint,
                    idempotency_key.as_deref(),
                    &preparation.task_id_hint,
                );
                return Err(store_error);
            }
            Err(message)
        }
    }
}

fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

async fn handle_stream(stream: UnixStream, state: Arc<AppState>) {
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
            if let Err(error) = fs::set_permissions(&socket_path, fs::Permissions::from_mode(0o600))
            {
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
            let skills_catalog = match load_skill_catalog(&daemon_config.skills) {
                Ok(catalog) => catalog,
                Err(message) => {
                    eprintln!("daemon_error=skills_load_failed message={}", message);
                    std::process::exit(1);
                }
            };
            let mcp_registry = match McpRegistry::from_config(&daemon_config.mcp) {
                Ok(registry) => registry,
                Err(message) => {
                    eprintln!("daemon_error=mcp_config_invalid message={}", message);
                    std::process::exit(1);
                }
            };
            if let Err(message) = store.prune_mcp_enabled_overrides(&mcp_registry.server_ids()) {
                eprintln!("daemon_error=store_open_failed message={}", message);
                std::process::exit(1);
            }
            let state = Arc::new(AppState {
                client: StubClient,
                store: Mutex::new(store),
                daemon_kernel: DaemonKernel::new(&kernel_config),
                skills_catalog,
                mcp_registry,
                model_config: daemon_config.model.clone(),
            });
            let mut handlers = JoinSet::new();
            let mut shutdown_requested = false;

            loop {
                if shutdown_requested && handlers.is_empty() {
                    break;
                }
                tokio::select! {
                    accept_result = listener.accept(), if !shutdown_requested => {
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

                        handlers.spawn(handle_stream(stream, Arc::clone(&state)));
                    }
                    joined = handlers.join_next(), if !handlers.is_empty() => {
                        if let Some(Err(error)) = joined {
                            eprintln!("daemon_error=request_handler_failed message={}", error);
                        }
                    }
                    _ = tokio::signal::ctrl_c(), if !shutdown_requested => {
                        shutdown_requested = true;
                    }
                }
            }

            let _ = std::fs::remove_file(&socket_path);
            println!("daemon_stopped");
        }
    }
}
