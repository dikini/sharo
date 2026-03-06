#[cfg(test)]
use std::cell::Cell;
use std::collections::BTreeMap;
use std::fs;
#[cfg(unix)]
use std::fs::OpenOptions;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(not(unix))]
compile_error!("sharo-daemon store persistence currently supports unix targets only");

use serde::{Deserialize, Serialize};
use sharo_core::protocol::{
    ApprovalSummary, ArtifactSummary, ListPendingApprovalsResponse, ResolveApprovalResponse,
    SubmitTaskOpRequest, SubmitTaskOpResponse, TaskSummary, TraceEventSummary, TraceSummary,
};
use sharo_core::reasoning_context::FitLoopRecord;
use sharo_core::runtime_types::{BindingRecord, BindingVisibility};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SessionRecord {
    session_id: String,
    session_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
struct PersistedState {
    sessions: BTreeMap<String, SessionRecord>,
    tasks: BTreeMap<String, TaskSummary>,
    traces: BTreeMap<String, TraceSummary>,
    artifacts: BTreeMap<String, Vec<ArtifactSummary>>,
    bindings: BTreeMap<String, Vec<BindingRecord>>,
    approvals: BTreeMap<String, ApprovalRecord>,
    resource_claims: BTreeMap<String, Vec<String>>,
    idempotency_keys: BTreeMap<String, String>,
    idempotency_failures: BTreeMap<String, String>,
    in_flight_idempotency_keys: BTreeMap<String, InFlightIdempotencyReservation>,
    next_session_id: u64,
    next_task_id: u64,
    next_turn_id_by_session: BTreeMap<String, u64>,
    next_approval_id: u64,
    next_conflict_id: u64,
    next_binding_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct InFlightIdempotencyReservation {
    task_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ApprovalRecord {
    approval_id: String,
    task_id: String,
    state: String,
    reason: String,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            sessions: BTreeMap::new(),
            tasks: BTreeMap::new(),
            traces: BTreeMap::new(),
            artifacts: BTreeMap::new(),
            bindings: BTreeMap::new(),
            approvals: BTreeMap::new(),
            resource_claims: BTreeMap::new(),
            idempotency_keys: BTreeMap::new(),
            idempotency_failures: BTreeMap::new(),
            in_flight_idempotency_keys: BTreeMap::new(),
            next_session_id: 1,
            next_task_id: 1,
            next_turn_id_by_session: BTreeMap::new(),
            next_approval_id: 1,
            next_conflict_id: 1,
            next_binding_id: 1,
        }
    }
}

pub struct Store {
    path: PathBuf,
    state: PersistedState,
}

pub enum SubmitReplay {
    Task(SubmitTaskOpResponse),
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmitPreparation {
    pub task_id_hint: String,
    pub task_id_sequence_hint: u64,
    pub session_id_hint: String,
    pub turn_id_hint: u64,
}

pub enum SubmitPreparationOutcome {
    Replay(SubmitReplay),
    Ready(SubmitPreparation),
}

enum SaveStateOutcome {
    Clean,
    DurabilityError(String),
}

impl Store {
    pub fn prepare_submit(
        &mut self,
        request: &SubmitTaskOpRequest,
    ) -> Result<SubmitPreparationOutcome, String> {
        let session_id_hint = request
            .session_id
            .clone()
            .unwrap_or_else(|| "session-implicit".to_string());
        if let Some(replay) =
            self.replay_by_idempotency(&session_id_hint, request.idempotency_key.as_deref())?
        {
            return Ok(SubmitPreparationOutcome::Replay(replay));
        }

        let idempotency_key = request.idempotency_key.as_deref();
        self.commit_mutation(|state| {
            let task_id_sequence_hint = state.next_task_id;
            state.next_task_id += 1;
            let turn_id_hint = reserve_next_turn_hint(state, &session_id_hint);
            let task_id_hint = format!("task-{:06}", task_id_sequence_hint);

            if let Some(key) = idempotency_key {
                let namespaced = namespaced_idempotency_key(&session_id_hint, key);
                state.in_flight_idempotency_keys.insert(
                    namespaced,
                    InFlightIdempotencyReservation {
                        task_id: task_id_hint.clone(),
                    },
                );
            }

            Ok(SubmitPreparationOutcome::Ready(SubmitPreparation {
                task_id_hint,
                task_id_sequence_hint,
                session_id_hint: session_id_hint.clone(),
                turn_id_hint,
            }))
        })
    }

    pub fn replay_by_idempotency(
        &self,
        session_id: &str,
        idempotency_key: Option<&str>,
    ) -> Result<Option<SubmitReplay>, String> {
        let Some(key) = idempotency_key else {
            return Ok(None);
        };
        let namespaced = namespaced_idempotency_key(session_id, key);
        if let Some(message) = self.state.idempotency_failures.get(&namespaced) {
            return Ok(Some(SubmitReplay::Error(message.clone())));
        }
        if let Some(reservation) = self.state.in_flight_idempotency_keys.get(&namespaced) {
            return Ok(Some(SubmitReplay::Error(format!(
                "submit_in_progress task_id={}",
                reservation.task_id
            ))));
        }
        let Some(existing_task_id) = self.state.idempotency_keys.get(&namespaced) else {
            return Ok(None);
        };
        let existing_task = self
            .state
            .tasks
            .get(existing_task_id)
            .ok_or_else(|| format!("idempotency_task_missing task_id={}", existing_task_id))?;
        Ok(Some(SubmitReplay::Task(SubmitTaskOpResponse {
            task_id: existing_task.task_id.clone(),
            task_state: existing_task.task_state.clone(),
            summary: "task replayed by idempotency key".to_string(),
        })))
    }

    pub fn record_submission_failure(
        &mut self,
        session_id: &str,
        idempotency_key: Option<&str>,
        message: &str,
    ) -> Result<(), String> {
        let Some(key) = idempotency_key else {
            return Ok(());
        };
        let namespaced = namespaced_idempotency_key(session_id, key);
        self.commit_mutation(|state| {
            state.in_flight_idempotency_keys.remove(&namespaced);
            state
                .idempotency_failures
                .insert(namespaced, message.to_string());
            Ok(())
        })
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref().to_path_buf();
        if !path.exists() {
            return Ok(Self {
                path,
                state: PersistedState::default(),
            });
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| format!("store_read_failed path={} error={}", path.display(), e))?;

        let state = serde_json::from_str::<PersistedState>(&content)
            .map_err(|e| format!("store_parse_failed path={} error={}", path.display(), e))?;
        let mut store = Self { path, state };
        if recover_stale_in_flight_idempotency(&mut store.state) {
            match store.save_state(&store.state)? {
                SaveStateOutcome::Clean => {}
                SaveStateOutcome::DurabilityError(message) => return Err(message),
            }
        }
        Ok(store)
    }

    fn save_state(&self, state: &PersistedState) -> Result<SaveStateOutcome, String> {
        let data = serde_json::to_string_pretty(state)
            .map_err(|e| format!("store_serialize_failed error={}", e))?;
        let parent = self
            .path
            .parent()
            .ok_or_else(|| format!("store_parent_missing path={}", self.path.display()))?;
        let file_name = self
            .path
            .file_name()
            .and_then(|v| v.to_str())
            .ok_or_else(|| format!("store_filename_invalid path={}", self.path.display()))?;
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("store_time_failed error={}", e))?
            .as_nanos();
        let tmp_path = parent.join(format!(
            ".{}.tmp-{}-{}",
            file_name,
            std::process::id(),
            nanos
        ));

        #[cfg(unix)]
        {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&tmp_path)
                .map_err(|e| {
                    format!("store_open_failed path={} error={}", tmp_path.display(), e)
                })?;
            file.write_all(data.as_bytes()).map_err(|e| {
                format!("store_write_failed path={} error={}", tmp_path.display(), e)
            })?;
            file.sync_all().map_err(|e| {
                format!("store_sync_failed path={} error={}", tmp_path.display(), e)
            })?;
            fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o600)).map_err(|e| {
                format!("store_chmod_failed path={} error={}", tmp_path.display(), e)
            })?;
            fs::rename(&tmp_path, &self.path).map_err(|e| {
                format!(
                    "store_rename_failed src={} dst={} error={}",
                    tmp_path.display(),
                    self.path.display(),
                    e
                )
            })?;
            match sync_directory(parent) {
                Ok(()) => Ok(SaveStateOutcome::Clean),
                Err(message) => Ok(SaveStateOutcome::DurabilityError(message)),
            }
        }
    }

    fn commit_mutation<R>(
        &mut self,
        mutate: impl FnOnce(&mut PersistedState) -> Result<R, String>,
    ) -> Result<R, String> {
        let mut next_state = self.state.clone();
        let result = mutate(&mut next_state)?;
        let save_outcome = self.save_state(&next_state)?;
        self.state = next_state;
        match save_outcome {
            SaveStateOutcome::Clean => Ok(result),
            SaveStateOutcome::DurabilityError(message) => Err(message),
        }
    }

    pub fn register_session(&mut self, session_label: &str) -> Result<String, String> {
        self.commit_mutation(|state| {
            let session_id = format!("session-{:06}", state.next_session_id);
            state.next_session_id += 1;

            state.sessions.insert(
                session_id.clone(),
                SessionRecord {
                    session_id: session_id.clone(),
                    session_label: session_label.to_string(),
                },
            );

            Ok(session_id)
        })
    }

    pub fn submit_task_with_route(
        &mut self,
        preparation: &SubmitPreparation,
        request: SubmitTaskOpRequest,
        route_decision_details: &str,
        model_output_text: &str,
        fit_loop_records: &[FitLoopRecord],
    ) -> Result<SubmitTaskOpResponse, String> {
        let session_id = preparation.session_id_hint.clone();
        let namespaced_idempotency_key = request
            .idempotency_key
            .as_deref()
            .map(|key| namespaced_idempotency_key(&session_id, key));
        if !self.owns_in_flight_idempotency(
            namespaced_idempotency_key.as_deref(),
            &preparation.task_id_hint,
        ) && let Some(replay) =
            self.replay_by_idempotency(&session_id, request.idempotency_key.as_deref())?
        {
            return match replay {
                SubmitReplay::Task(response) => Ok(response),
                SubmitReplay::Error(message) => Err(format!(
                    "idempotency_failure_replay_unexpected message={message}"
                )),
            };
        }
        self.commit_mutation(|state| {
            let task_id = preparation.task_id_hint.clone();
            state.next_task_id = state
                .next_task_id
                .max(preparation.task_id_sequence_hint.saturating_add(1));

            let invalid_manifest = request.goal.contains("invalid_manifest:");
            let restricted = request.goal.contains("restricted:");
            let resource = parse_resource_claim(&request.goal);
            let step_id = format!("step-{}", task_id);

            let mut task = TaskSummary {
                task_id: task_id.clone(),
                session_id: session_id.clone(),
                task_state: if invalid_manifest {
                    "blocked".to_string()
                } else if restricted {
                    "awaiting_approval".to_string()
                } else {
                    "succeeded".to_string()
                },
                current_step_summary: "read one context item".to_string(),
                blocking_reason: None,
                coordination_summary: None,
                result_preview: if invalid_manifest || restricted {
                    None
                } else {
                    Some(summarize_model_output(model_output_text))
                },
            };

            if invalid_manifest {
                task.current_step_summary = "capability manifest validation failed".to_string();
                task.blocking_reason = Some("manifest_invalid".to_string());
            } else if restricted {
                let approval_id = format!("approval-{:06}", state.next_approval_id);
                state.next_approval_id += 1;
                state.approvals.insert(
                    approval_id.clone(),
                    ApprovalRecord {
                        approval_id: approval_id.clone(),
                        task_id: task_id.clone(),
                        state: "pending".to_string(),
                        reason: "policy require_approval".to_string(),
                    },
                );
                task.current_step_summary =
                    "awaiting approval for restricted capability".to_string();
                task.blocking_reason = Some(format!("approval_required approval_id={approval_id}"));
            }

            let mut trace = TraceSummary {
                trace_id: format!("trace-{}", task_id),
                task_id: task_id.clone(),
                session_id: session_id.clone(),
                events: vec![
                    TraceEventSummary {
                        event_sequence: 1,
                        event_kind: "task_submitted".to_string(),
                        details: request.goal.clone(),
                    },
                    TraceEventSummary {
                        event_sequence: 2,
                        event_kind: "route_decision".to_string(),
                        details: route_decision_details.to_string(),
                    },
                ],
            };
            for record in fit_loop_records {
                let event_kind = if record.decision == "fitted" {
                    "fit_loop_fitted"
                } else {
                    "fit_loop_adjusted"
                };
                let details = format!(
                    "iteration={} plan_id={} before_hash={} after_hash={}",
                    record.iteration,
                    record.plan_id.as_deref().unwrap_or("none"),
                    record.before_state_hash.as_deref().unwrap_or("none"),
                    record.after_state_hash.as_deref().unwrap_or("none"),
                );
                push_trace_event(&mut trace, event_kind, &details);
            }
            push_trace_event(
                &mut trace,
                "model_output_received",
                &summarize_model_output(model_output_text),
            );
            push_trace_event(
                &mut trace,
                "verification_completed",
                "postconditions_satisfied",
            );

            let mut task_bindings: Vec<BindingRecord> = Vec::new();
            if !invalid_manifest {
                if restricted {
                    let binding = new_binding(
                        state,
                        &task_id,
                        &step_id,
                        BindingVisibility::ApprovalGated,
                        "approval-handle",
                        None,
                    );
                    push_trace_event(
                        &mut trace,
                        "binding_created",
                        &format!(
                            "binding_id={} visibility=approval_gated",
                            binding.binding_id
                        ),
                    );
                    push_trace_event(
                        &mut trace,
                        "binding_redacted_for_model",
                        &format!("binding_id={}", binding.binding_id),
                    );
                    task_bindings.push(binding);
                } else {
                    let binding = new_binding(
                        state,
                        &task_id,
                        &step_id,
                        BindingVisibility::EngineOnly,
                        "engine-handle",
                        None,
                    );
                    push_trace_event(
                        &mut trace,
                        "binding_created",
                        &format!("binding_id={} visibility=engine_only", binding.binding_id),
                    );
                    push_trace_event(
                        &mut trace,
                        "binding_redacted_for_model",
                        &format!("binding_id={}", binding.binding_id),
                    );
                    task_bindings.push(binding);
                }
            }

            if invalid_manifest {
                push_trace_event(
                    &mut trace,
                    "manifest_validation_failed",
                    "missing or invalid capability manifest",
                );
            } else if restricted {
                push_trace_event(&mut trace, "policy_decision", "require_approval");
                push_trace_event(&mut trace, "approval_requested", "pending");
            }

            if let Some(resource_key) = resource {
                let claim_entry = state
                    .resource_claims
                    .entry(resource_key.clone())
                    .or_default();
                if !claim_entry.is_empty() {
                    let conflict_id = format!("conflict-{:06}", state.next_conflict_id);
                    state.next_conflict_id += 1;
                    task.coordination_summary = Some(format!(
                        "conflict_detected conflict_id={} resource={}",
                        conflict_id, resource_key
                    ));
                    push_trace_event(
                        &mut trace,
                        "conflict_detected",
                        &format!(
                            "resource={} related_tasks={}",
                            resource_key,
                            claim_entry.join(",")
                        ),
                    );
                }
                claim_entry.push(task_id.clone());
            }

            let mut artifacts = vec![
                ArtifactSummary {
                    artifact_id: format!("artifact-{}-route", task_id),
                    artifact_kind: "route_decision".to_string(),
                    summary: if route_decision_details == "local_mock" {
                        "selected local mock route".to_string()
                    } else {
                        format!("selected {} route", route_decision_details)
                    },
                    produced_by_step_id: step_id.clone(),
                    produced_by_trace_event_sequence: event_sequence_by_kind(
                        &trace,
                        "route_decision",
                    ),
                },
                ArtifactSummary {
                    artifact_id: format!("artifact-{}-fit-loop", task_id),
                    artifact_kind: "fit_loop_decision".to_string(),
                    summary: summarize_fit_loop(fit_loop_records),
                    produced_by_step_id: step_id.clone(),
                    produced_by_trace_event_sequence: event_sequence_by_kind(
                        &trace,
                        if fit_loop_records.last().map(|r| r.decision.as_str()) == Some("adjusted")
                        {
                            "fit_loop_adjusted"
                        } else {
                            "fit_loop_fitted"
                        },
                    ),
                },
                ArtifactSummary {
                    artifact_id: format!("artifact-{}-model-output", task_id),
                    artifact_kind: "model_output".to_string(),
                    summary: summarize_model_output(model_output_text),
                    produced_by_step_id: step_id.clone(),
                    produced_by_trace_event_sequence: event_sequence_by_kind(
                        &trace,
                        "model_output_received",
                    ),
                },
                ArtifactSummary {
                    artifact_id: format!("artifact-{}-verification", task_id),
                    artifact_kind: "verification_result".to_string(),
                    summary: "postconditions satisfied".to_string(),
                    produced_by_step_id: step_id.clone(),
                    produced_by_trace_event_sequence: event_sequence_by_kind(
                        &trace,
                        "verification_completed",
                    ),
                },
            ];
            if task.task_state == "succeeded" {
                artifacts.push(ArtifactSummary {
                    artifact_id: format!("artifact-{}-final", task_id),
                    artifact_kind: "final_result".to_string(),
                    summary: "task succeeded".to_string(),
                    produced_by_step_id: step_id.clone(),
                    produced_by_trace_event_sequence: event_sequence_by_kind(
                        &trace,
                        "verification_completed",
                    ),
                });
            }
            if invalid_manifest {
                artifacts.push(ArtifactSummary {
                    artifact_id: format!("artifact-{}-manifest", task_id),
                    artifact_kind: "failure_record".to_string(),
                    summary: "capability manifest invalid".to_string(),
                    produced_by_step_id: step_id.clone(),
                    produced_by_trace_event_sequence: event_sequence_by_kind(
                        &trace,
                        "manifest_validation_failed",
                    ),
                });
            } else if restricted {
                artifacts.push(ArtifactSummary {
                    artifact_id: format!("artifact-{}-approval", task_id),
                    artifact_kind: "verification_result".to_string(),
                    summary: "restricted step is approval gated".to_string(),
                    produced_by_step_id: step_id.clone(),
                    produced_by_trace_event_sequence: event_sequence_by_kind(
                        &trace,
                        "approval_requested",
                    ),
                });
            }
            if task.coordination_summary.is_some() {
                artifacts.push(ArtifactSummary {
                    artifact_id: format!("artifact-{}-coordination", task_id),
                    artifact_kind: "verification_result".to_string(),
                    summary: "coordination summary recorded".to_string(),
                    produced_by_step_id: step_id.clone(),
                    produced_by_trace_event_sequence: event_sequence_by_kind(
                        &trace,
                        "conflict_detected",
                    ),
                });
            }

            state.tasks.insert(task_id.clone(), task);
            state.traces.insert(task_id.clone(), trace);
            state.artifacts.insert(task_id.clone(), artifacts);
            state.bindings.insert(task_id.clone(), task_bindings);
            if let Some(idempotency_key) = namespaced_idempotency_key.clone() {
                state.in_flight_idempotency_keys.remove(&idempotency_key);
                state
                    .idempotency_keys
                    .insert(idempotency_key, task_id.clone());
            }

            let response_state = state
                .tasks
                .get(&task_id)
                .map(|t| t.task_state.clone())
                .unwrap_or_else(|| "succeeded".to_string());

            Ok(SubmitTaskOpResponse {
                task_id,
                task_state: response_state,
                summary: "task accepted".to_string(),
            })
        })
    }

    pub fn submit_failed_task(
        &mut self,
        preparation: &SubmitPreparation,
        request: SubmitTaskOpRequest,
        failure_message: &str,
        fit_loop_records: &[FitLoopRecord],
    ) -> Result<SubmitTaskOpResponse, String> {
        let session_id = preparation.session_id_hint.clone();
        let namespaced_idempotency_key = request
            .idempotency_key
            .as_deref()
            .map(|key| namespaced_idempotency_key(&session_id, key));
        if !self.owns_in_flight_idempotency(
            namespaced_idempotency_key.as_deref(),
            &preparation.task_id_hint,
        ) && let Some(replay) =
            self.replay_by_idempotency(&session_id, request.idempotency_key.as_deref())?
        {
            return match replay {
                SubmitReplay::Task(response) => Ok(response),
                SubmitReplay::Error(message) => Err(format!(
                    "idempotency_failure_replay_unexpected message={message}"
                )),
            };
        }
        self.commit_mutation(|state| {
            let task_id = preparation.task_id_hint.clone();
            state.next_task_id = state
                .next_task_id
                .max(preparation.task_id_sequence_hint.saturating_add(1));
            let step_id = format!("step-{}", task_id);

            let task = TaskSummary {
                task_id: task_id.clone(),
                session_id: session_id.clone(),
                task_state: "failed".to_string(),
                current_step_summary: "reasoning policy fit failed".to_string(),
                blocking_reason: Some(failure_message.to_string()),
                coordination_summary: None,
                result_preview: None,
            };

            let mut trace = TraceSummary {
                trace_id: format!("trace-{}", task_id),
                task_id: task_id.clone(),
                session_id: session_id.clone(),
                events: vec![TraceEventSummary {
                    event_sequence: 1,
                    event_kind: "task_submitted".to_string(),
                    details: request.goal.clone(),
                }],
            };
            for record in fit_loop_records {
                let event_kind = if record.decision == "fitted" {
                    "fit_loop_fitted"
                } else {
                    "fit_loop_adjusted"
                };
                let details = format!(
                    "iteration={} plan_id={} before_hash={} after_hash={}",
                    record.iteration,
                    record.plan_id.as_deref().unwrap_or("none"),
                    record.before_state_hash.as_deref().unwrap_or("none"),
                    record.after_state_hash.as_deref().unwrap_or("none"),
                );
                push_trace_event(&mut trace, event_kind, &details);
            }
            push_trace_event(&mut trace, "fit_loop_failed", failure_message);

            let artifacts = vec![
                ArtifactSummary {
                    artifact_id: format!("artifact-{}-fit-loop", task_id),
                    artifact_kind: "fit_loop_decision".to_string(),
                    summary: summarize_fit_loop_failure(fit_loop_records),
                    produced_by_step_id: step_id.clone(),
                    produced_by_trace_event_sequence: event_sequence_by_kind(
                        &trace,
                        "fit_loop_failed",
                    ),
                },
                ArtifactSummary {
                    artifact_id: format!("artifact-{}-failure", task_id),
                    artifact_kind: "failure_record".to_string(),
                    summary: failure_message.to_string(),
                    produced_by_step_id: step_id,
                    produced_by_trace_event_sequence: event_sequence_by_kind(
                        &trace,
                        "fit_loop_failed",
                    ),
                },
            ];

            state.tasks.insert(task_id.clone(), task);
            state.traces.insert(task_id.clone(), trace);
            state.artifacts.insert(task_id.clone(), artifacts);
            state.bindings.insert(task_id.clone(), Vec::new());
            if let Some(idempotency_key) = namespaced_idempotency_key.clone() {
                state.in_flight_idempotency_keys.remove(&idempotency_key);
                state
                    .idempotency_keys
                    .insert(idempotency_key, task_id.clone());
            }

            Ok(SubmitTaskOpResponse {
                task_id,
                task_state: "failed".to_string(),
                summary: failure_message.to_string(),
            })
        })
    }

    pub fn get_task(&self, task_id: &str) -> Option<TaskSummary> {
        self.state.tasks.get(task_id).cloned()
    }

    fn owns_in_flight_idempotency(
        &self,
        namespaced_idempotency_key: Option<&str>,
        task_id: &str,
    ) -> bool {
        namespaced_idempotency_key
            .and_then(|key| self.state.in_flight_idempotency_keys.get(key))
            .is_some_and(|reservation| reservation.task_id == task_id)
    }

    pub fn get_trace(&self, task_id: &str) -> Option<TraceSummary> {
        self.state.traces.get(task_id).cloned()
    }

    pub fn get_artifacts(&self, task_id: &str) -> Vec<ArtifactSummary> {
        self.state
            .artifacts
            .get(task_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn list_pending_approvals(&self) -> ListPendingApprovalsResponse {
        ListPendingApprovalsResponse {
            approvals: self
                .state
                .approvals
                .values()
                .filter(|a| a.state == "pending")
                .map(|a| ApprovalSummary {
                    approval_id: a.approval_id.clone(),
                    task_id: a.task_id.clone(),
                    state: a.state.clone(),
                    reason: a.reason.clone(),
                })
                .collect(),
        }
    }

    pub fn resolve_approval(
        &mut self,
        approval_id: &str,
        decision: &str,
    ) -> Result<ResolveApprovalResponse, String> {
        if decision != "approve" && decision != "deny" {
            return Err(format!(
                "approval_decision_invalid decision={} expected=approve|deny",
                decision
            ));
        }
        self.commit_mutation(|state| {
            let (task_id, final_state, response_approval_id) = {
                let approval = state
                    .approvals
                    .get_mut(approval_id)
                    .ok_or_else(|| format!("approval_not_found approval_id={approval_id}"))?;

                if approval.state != "pending" {
                    return Ok(ResolveApprovalResponse {
                        approval_id: approval.approval_id.clone(),
                        task_id: approval.task_id.clone(),
                        state: approval.state.clone(),
                    });
                }

                approval.state = if decision == "approve" {
                    "approved".to_string()
                } else {
                    "denied".to_string()
                };
                (
                    approval.task_id.clone(),
                    approval.state.clone(),
                    approval.approval_id.clone(),
                )
            };

            if final_state == "approved" {
                let result_preview = state.artifacts.get(&task_id).and_then(|artifacts| {
                    artifacts
                        .iter()
                        .find(|artifact| artifact.artifact_kind == "model_output")
                        .map(|artifact| artifact.summary.clone())
                });
                if let Some(task) = state.tasks.get_mut(&task_id) {
                    task.task_state = "succeeded".to_string();
                    task.current_step_summary =
                        "restricted step approved and completed".to_string();
                    task.blocking_reason = None;
                    if task.result_preview.is_none() {
                        task.result_preview = result_preview;
                    }
                }
            } else if let Some(task) = state.tasks.get_mut(&task_id) {
                task.task_state = "blocked".to_string();
                task.current_step_summary = "restricted step denied".to_string();
                task.blocking_reason = Some("approval_denied".to_string());
            }

            if final_state == "approved" {
                let final_sequence = state
                    .traces
                    .get(&task_id)
                    .map(|trace| trace.events.len() as u64 + 1)
                    .unwrap_or(1);
                let artifacts = state.artifacts.entry(task_id.clone()).or_default();
                let has_final = artifacts.iter().any(|a| a.artifact_kind == "final_result");
                if !has_final {
                    artifacts.push(ArtifactSummary {
                        artifact_id: format!("artifact-{}-final", task_id),
                        artifact_kind: "final_result".to_string(),
                        summary: "task succeeded".to_string(),
                        produced_by_step_id: format!("step-{}", task_id),
                        produced_by_trace_event_sequence: final_sequence,
                    });
                }
            }

            if let Some(trace) = state.traces.get_mut(&task_id) {
                trace.events.push(TraceEventSummary {
                    event_sequence: (trace.events.len() as u64) + 1,
                    event_kind: "approval_resolved".to_string(),
                    details: final_state.clone(),
                });
            }

            Ok(ResolveApprovalResponse {
                approval_id: response_approval_id,
                task_id,
                state: final_state,
            })
        })
    }
}

fn namespaced_idempotency_key(session_id: &str, key: &str) -> String {
    format!("{session_id}:{key}")
}

fn next_turn_id_for_session(state: &PersistedState, session_id: &str) -> u64 {
    state
        .next_turn_id_by_session
        .get(session_id)
        .copied()
        .unwrap_or_else(|| {
            state
                .tasks
                .values()
                .filter(|task| task.session_id == session_id)
                .count() as u64
                + 1
        })
}

fn reserve_next_turn_hint(state: &mut PersistedState, session_id: &str) -> u64 {
    let next_turn = next_turn_id_for_session(state, session_id);
    state
        .next_turn_id_by_session
        .insert(session_id.to_string(), next_turn + 1);
    next_turn
}

fn recover_stale_in_flight_idempotency(state: &mut PersistedState) -> bool {
    if state.in_flight_idempotency_keys.is_empty() {
        return false;
    }

    for (namespaced_key, reservation) in std::mem::take(&mut state.in_flight_idempotency_keys) {
        state
            .idempotency_failures
            .entry(namespaced_key)
            .or_insert_with(|| {
                format!(
                    "submit_interrupted_by_restart task_id={}",
                    reservation.task_id
                )
            });
    }
    true
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<(), String> {
    #[cfg(test)]
    if directory_sync_should_fail_for_test() {
        return Err(format!(
            "store_directory_sync_failed path={} error=simulated_failure",
            path.display()
        ));
    }
    let directory = OpenOptions::new().read(true).open(path).map_err(|e| {
        format!(
            "store_directory_sync_failed path={} error={}",
            path.display(),
            e
        )
    })?;
    directory.sync_all().map_err(|e| {
        format!(
            "store_directory_sync_failed path={} error={}",
            path.display(),
            e
        )
    })
}

#[cfg(test)]
thread_local! {
    static DIRECTORY_SYNC_FAIL_FOR_TEST: Cell<bool> = const { Cell::new(false) };
}

#[cfg(test)]
fn directory_sync_should_fail_for_test() -> bool {
    DIRECTORY_SYNC_FAIL_FOR_TEST.with(Cell::get)
}

#[cfg(test)]
fn with_directory_sync_failure_for_test<T>(f: impl FnOnce() -> T) -> T {
    struct ResetGuard(bool);

    impl Drop for ResetGuard {
        fn drop(&mut self) {
            DIRECTORY_SYNC_FAIL_FOR_TEST.with(|flag| flag.set(self.0));
        }
    }

    DIRECTORY_SYNC_FAIL_FOR_TEST.with(|flag| {
        let previous = flag.replace(true);
        let _reset = ResetGuard(previous);
        f()
    })
}

fn parse_resource_claim(goal: &str) -> Option<String> {
    goal.split_whitespace()
        .find_map(|token| token.strip_prefix("resource:"))
        .map(|v| v.to_string())
}

fn push_trace_event(trace: &mut TraceSummary, event_kind: &str, details: &str) {
    trace.events.push(TraceEventSummary {
        event_sequence: (trace.events.len() as u64) + 1,
        event_kind: event_kind.to_string(),
        details: details.to_string(),
    });
}

fn event_sequence_by_kind(trace: &TraceSummary, event_kind: &str) -> u64 {
    trace
        .events
        .iter()
        .rev()
        .find(|e| e.event_kind == event_kind)
        .map(|e| e.event_sequence)
        .unwrap_or(0)
}

fn summarize_model_output(content: &str) -> String {
    const LIMIT: usize = 240;
    let trimmed = content.trim();
    if trimmed.len() <= LIMIT {
        return trimmed.to_string();
    }

    let mut snippet = trimmed.chars().take(LIMIT).collect::<String>();
    snippet.push_str("...");
    snippet
}

fn summarize_fit_loop(records: &[FitLoopRecord]) -> String {
    if records.is_empty() {
        return "fit_loop_no_records".to_string();
    }
    let iterations = records.len();
    let final_decision = records
        .last()
        .map(|r| r.decision.as_str())
        .unwrap_or("unknown");
    format!(
        "fit_loop iterations={} final_decision={}",
        iterations, final_decision
    )
}

fn summarize_fit_loop_failure(records: &[FitLoopRecord]) -> String {
    if records.is_empty() {
        return "fit_loop iterations=0 final_decision=failed".to_string();
    }
    format!(
        "fit_loop iterations={} final_decision=failed",
        records.len()
    )
}

fn new_binding(
    state: &mut PersistedState,
    task_id: &str,
    step_id: &str,
    visibility: BindingVisibility,
    handle: &str,
    raw_value_model_text: Option<String>,
) -> BindingRecord {
    let binding_id = format!("binding-{:06}", state.next_binding_id);
    state.next_binding_id += 1;
    let exposed = raw_value_model_text.is_some();
    BindingRecord {
        binding_id,
        task_id: task_id.to_string(),
        step_id: step_id.to_string(),
        visibility,
        handle: handle.to_string(),
        raw_value_model_text,
        raw_value_redacted: !exposed,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ApprovalRecord, PersistedState, Store, SubmitPreparation, SubmitReplay, sync_directory,
        with_directory_sync_failure_for_test,
    };
    use sharo_core::protocol::{SubmitTaskOpRequest, TaskSummary, TraceSummary};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_store_path(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{nanos}.json"))
    }

    fn store_with_failing_save() -> Store {
        let missing_parent = std::env::temp_dir().join(format!(
            "sharo-missing-parent-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        Store {
            path: missing_parent.join("store.json"),
            state: PersistedState::default(),
        }
    }

    fn assert_save_failed(error: &str) {
        assert!(
            error.contains("store_parent_missing") || error.contains("store_open_failed"),
            "unexpected save error: {error}"
        );
    }

    #[test]
    fn replay_by_idempotency_returns_persisted_submission_error() {
        let path = unique_store_path("sharo-store-idempotency-failure");
        let mut store = Store::open(&path).expect("open store");
        store
            .record_submission_failure(
                "session-000001",
                Some("idem-1"),
                "missing auth env var SHARO_TEST_MISSING_OPENAI_KEY",
            )
            .expect("record failure");

        let replay = store
            .replay_by_idempotency("session-000001", Some("idem-1"))
            .expect("replay")
            .expect("replay result");
        match replay {
            SubmitReplay::Error(message) => {
                assert!(message.contains("missing auth env var SHARO_TEST_MISSING_OPENAI_KEY"));
            }
            SubmitReplay::Task(response) => panic!("unexpected task replay: {response:?}"),
        }

        let _ = fs::remove_file(path);
    }

    #[test]
    fn register_session_rolls_back_when_save_fails() {
        let mut store = store_with_failing_save();
        let before = store.state.clone();

        let error = store
            .register_session("session-label")
            .expect_err("save failure");

        assert_save_failed(&error);
        assert_eq!(store.state, before);
    }

    #[test]
    fn submit_task_rolls_back_when_save_fails() {
        let mut store = store_with_failing_save();
        let before = store.state.clone();
        let preparation = SubmitPreparation {
            task_id_hint: "task-000001".to_string(),
            task_id_sequence_hint: 1,
            session_id_hint: "session-000001".to_string(),
            turn_id_hint: 1,
        };

        let error = store
            .submit_task_with_route(
                &preparation,
                SubmitTaskOpRequest {
                    session_id: Some("session-000001".to_string()),
                    goal: "read one context item".to_string(),
                    idempotency_key: Some("idem-rollback".to_string()),
                },
                "local_mock",
                "deterministic-response",
                &[],
            )
            .expect_err("save failure");

        assert_save_failed(&error);
        assert_eq!(store.state, before);
    }

    #[test]
    fn resolve_approval_rolls_back_when_save_fails() {
        let mut store = store_with_failing_save();
        store.state.tasks.insert(
            "task-000001".to_string(),
            TaskSummary {
                task_id: "task-000001".to_string(),
                session_id: "session-000001".to_string(),
                task_state: "awaiting_approval".to_string(),
                current_step_summary: "awaiting approval".to_string(),
                blocking_reason: Some("approval_required approval_id=approval-000001".to_string()),
                coordination_summary: None,
                result_preview: None,
            },
        );
        store.state.traces.insert(
            "task-000001".to_string(),
            TraceSummary {
                trace_id: "trace-task-000001".to_string(),
                task_id: "task-000001".to_string(),
                session_id: "session-000001".to_string(),
                events: Vec::new(),
            },
        );
        store.state.approvals.insert(
            "approval-000001".to_string(),
            ApprovalRecord {
                approval_id: "approval-000001".to_string(),
                task_id: "task-000001".to_string(),
                state: "pending".to_string(),
                reason: "policy require_approval".to_string(),
            },
        );
        let before = store.state.clone();

        let error = store
            .resolve_approval("approval-000001", "approve")
            .expect_err("save failure");

        assert_save_failed(&error);
        assert_eq!(store.state, before);
    }

    #[test]
    fn failed_store_mutation_preserves_pre_call_state() {
        let mut store = store_with_failing_save();
        let before = store.state.clone();

        let _ = store.record_submission_failure(
            "session-000001",
            Some("idem-rollback"),
            "connector failed",
        );

        assert_eq!(store.state, before);
    }

    #[test]
    fn sync_directory_accepts_existing_directory() {
        let directory = std::env::temp_dir();
        sync_directory(&directory).expect("sync existing directory");
    }

    #[test]
    fn sync_directory_rejects_missing_directory() {
        let missing_directory = std::env::temp_dir().join(format!(
            "sharo-missing-dir-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        let error = sync_directory(&missing_directory).expect_err("missing directory should fail");
        assert!(error.contains("store_directory_sync_failed"));
    }

    #[test]
    fn post_rename_directory_sync_failure_keeps_memory_and_disk_consistent() {
        let path = unique_store_path("sharo-post-rename-sync-failure");
        let mut store = Store::open(&path).expect("open store");

        with_directory_sync_failure_for_test(|| {
            let error = store
                .register_session("session-label")
                .expect_err("directory sync failure should surface");
            assert!(error.contains("store_directory_sync_failed"));
        });

        let reopened = Store::open(&path).expect("reopen store");
        assert_eq!(store.state, reopened.state);
        assert_eq!(store.state.next_session_id, 2);
        assert!(store.state.sessions.contains_key("session-000001"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn prepare_submit_reserves_unique_hints_under_concurrency() {
        let path = unique_store_path("sharo-prepare-submit-reservations");
        let mut store = Store::open(&path).expect("open store");

        let first = match store
            .prepare_submit(&SubmitTaskOpRequest {
                session_id: Some("session-000001".to_string()),
                goal: "first".to_string(),
                idempotency_key: None,
            })
            .expect("first prepare")
        {
            super::SubmitPreparationOutcome::Ready(preparation) => preparation,
            super::SubmitPreparationOutcome::Replay(_) => panic!("unexpected replay"),
        };

        let second = match store
            .prepare_submit(&SubmitTaskOpRequest {
                session_id: Some("session-000001".to_string()),
                goal: "second".to_string(),
                idempotency_key: None,
            })
            .expect("second prepare")
        {
            super::SubmitPreparationOutcome::Ready(preparation) => preparation,
            super::SubmitPreparationOutcome::Replay(_) => panic!("unexpected replay"),
        };

        assert_ne!(first.task_id_hint, second.task_id_hint);
        assert_ne!(first.turn_id_hint, second.turn_id_hint);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn prepare_submit_blocks_duplicate_inflight_idempotency_keys() {
        let path = unique_store_path("sharo-prepare-submit-idempotency");
        let mut store = Store::open(&path).expect("open store");
        let request = SubmitTaskOpRequest {
            session_id: Some("session-000001".to_string()),
            goal: "first".to_string(),
            idempotency_key: Some("idem-1".to_string()),
        };

        match store.prepare_submit(&request).expect("first prepare") {
            super::SubmitPreparationOutcome::Ready(_) => {}
            super::SubmitPreparationOutcome::Replay(_) => panic!("unexpected replay"),
        }
        assert_eq!(store.state.next_task_id, 2);
        assert_eq!(store.state.in_flight_idempotency_keys.len(), 1);

        match store
            .prepare_submit(&request)
            .expect("duplicate prepare should not fail")
        {
            super::SubmitPreparationOutcome::Replay(super::SubmitReplay::Error(message)) => {
                assert!(message.contains("submit_in_progress"));
            }
            super::SubmitPreparationOutcome::Replay(super::SubmitReplay::Task(_))
            | super::SubmitPreparationOutcome::Ready(_) => {
                panic!("unexpected duplicate prepare outcome")
            }
        }

        let _ = fs::remove_file(path);
    }

    #[test]
    fn reopened_store_keeps_reserved_identity_high_water_marks() {
        let path = unique_store_path("sharo-reopen-submit-reservation");
        let mut store = Store::open(&path).expect("open store");

        let first = match store
            .prepare_submit(&SubmitTaskOpRequest {
                session_id: Some("session-000001".to_string()),
                goal: "first".to_string(),
                idempotency_key: Some("idem-restart".to_string()),
            })
            .expect("first prepare")
        {
            super::SubmitPreparationOutcome::Ready(preparation) => preparation,
            super::SubmitPreparationOutcome::Replay(_) => panic!("unexpected replay"),
        };
        assert_eq!(store.state.next_task_id, 2);
        assert_eq!(
            store.state.next_turn_id_by_session.get("session-000001"),
            Some(&2)
        );

        drop(store);

        let mut reopened = Store::open(&path).expect("reopen store");
        match reopened
            .prepare_submit(&SubmitTaskOpRequest {
                session_id: Some("session-000001".to_string()),
                goal: "duplicate".to_string(),
                idempotency_key: Some("idem-restart".to_string()),
            })
            .expect("duplicate replay after reopen")
        {
            super::SubmitPreparationOutcome::Replay(super::SubmitReplay::Error(message)) => {
                assert!(message.contains("submit_interrupted_by_restart"));
                assert!(message.contains(&first.task_id_hint));
            }
            super::SubmitPreparationOutcome::Replay(super::SubmitReplay::Task(_))
            | super::SubmitPreparationOutcome::Ready(_) => {
                panic!("unexpected reopened duplicate outcome")
            }
        }
        let second = match reopened
            .prepare_submit(&SubmitTaskOpRequest {
                session_id: Some("session-000001".to_string()),
                goal: "second".to_string(),
                idempotency_key: None,
            })
            .expect("second prepare")
        {
            super::SubmitPreparationOutcome::Ready(preparation) => preparation,
            super::SubmitPreparationOutcome::Replay(_) => panic!("unexpected replay"),
        };

        assert_ne!(first.task_id_hint, second.task_id_hint);
        assert_ne!(first.turn_id_hint, second.turn_id_hint);
        assert_eq!(second.task_id_hint, "task-000002");
        assert_eq!(second.turn_id_hint, 2);

        let _ = fs::remove_file(path);
    }
}
