use std::collections::BTreeMap;
use std::fs;
#[cfg(unix)]
use std::fs::OpenOptions;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sharo_core::protocol::{
    ApprovalSummary, ArtifactSummary, ListPendingApprovalsResponse, ResolveApprovalResponse,
    SubmitTaskOpRequest, SubmitTaskOpResponse, TaskSummary, TraceEventSummary, TraceSummary,
};
use sharo_core::runtime_types::{BindingRecord, BindingVisibility};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionRecord {
    session_id: String,
    session_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedState {
    sessions: BTreeMap<String, SessionRecord>,
    tasks: BTreeMap<String, TaskSummary>,
    traces: BTreeMap<String, TraceSummary>,
    artifacts: BTreeMap<String, Vec<ArtifactSummary>>,
    bindings: BTreeMap<String, Vec<BindingRecord>>,
    approvals: BTreeMap<String, ApprovalRecord>,
    resource_claims: BTreeMap<String, Vec<String>>,
    idempotency_keys: BTreeMap<String, String>,
    next_session_id: u64,
    next_task_id: u64,
    next_approval_id: u64,
    next_conflict_id: u64,
    next_binding_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
            next_session_id: 1,
            next_task_id: 1,
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

impl Store {
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

        Ok(Self { path, state })
    }

    fn save(&self) -> Result<(), String> {
        let data = serde_json::to_string_pretty(&self.state)
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
        let tmp_path = parent.join(format!(".{}.tmp-{}-{}", file_name, std::process::id(), nanos));

        #[cfg(unix)]
        {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&tmp_path)
                .map_err(|e| format!("store_open_failed path={} error={}", tmp_path.display(), e))?;
            file.write_all(data.as_bytes())
                .map_err(|e| format!("store_write_failed path={} error={}", tmp_path.display(), e))?;
            file.sync_all()
                .map_err(|e| format!("store_sync_failed path={} error={}", tmp_path.display(), e))?;
            fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o600))
                .map_err(|e| format!("store_chmod_failed path={} error={}", tmp_path.display(), e))?;
            fs::rename(&tmp_path, &self.path).map_err(|e| {
                format!(
                    "store_rename_failed src={} dst={} error={}",
                    tmp_path.display(),
                    self.path.display(),
                    e
                )
            })?;
            fs::set_permissions(&self.path, fs::Permissions::from_mode(0o600))
                .map_err(|e| format!("store_chmod_failed path={} error={}", self.path.display(), e))
        }
        #[cfg(not(unix))]
        {
            {
                let mut file = fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&tmp_path)
                    .map_err(|e| format!("store_open_failed path={} error={}", tmp_path.display(), e))?;
                file.write_all(data.as_bytes()).map_err(|e| {
                    format!("store_write_failed path={} error={}", tmp_path.display(), e)
                })?;
                file.sync_all()
                    .map_err(|e| format!("store_sync_failed path={} error={}", tmp_path.display(), e))?;
            }
            fs::rename(&tmp_path, &self.path).map_err(|e| {
                format!(
                    "store_rename_failed src={} dst={} error={}",
                    tmp_path.display(),
                    self.path.display(),
                    e
                )
            })
        }
    }

    pub fn register_session(&mut self, session_label: &str) -> Result<String, String> {
        let session_id = format!("session-{:06}", self.state.next_session_id);
        self.state.next_session_id += 1;

        self.state.sessions.insert(
            session_id.clone(),
            SessionRecord {
                session_id: session_id.clone(),
                session_label: session_label.to_string(),
            },
        );

        self.save()?;
        Ok(session_id)
    }

    pub fn submit_task(&mut self, request: SubmitTaskOpRequest) -> Result<SubmitTaskOpResponse, String> {
        let session_id = request
            .session_id
            .clone()
            .unwrap_or_else(|| "session-implicit".to_string());
        let namespaced_idempotency_key = request
            .idempotency_key
            .as_deref()
            .map(|key| format!("{}:{}", session_id, key));

        if let Some(idempotency_key) = namespaced_idempotency_key.as_deref() {
            if let Some(existing_task_id) = self.state.idempotency_keys.get(idempotency_key) {
                let existing_task = self
                    .state
                    .tasks
                    .get(existing_task_id)
                    .ok_or_else(|| format!("idempotency_task_missing task_id={}", existing_task_id))?;
                return Ok(SubmitTaskOpResponse {
                    task_id: existing_task.task_id.clone(),
                    task_state: existing_task.task_state.clone(),
                    summary: "task replayed by idempotency key".to_string(),
                });
            }
        }

        let task_id = format!("task-{:06}", self.state.next_task_id);
        self.state.next_task_id += 1;

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
        };

        if invalid_manifest {
            task.current_step_summary = "capability manifest validation failed".to_string();
            task.blocking_reason = Some("manifest_invalid".to_string());
        } else if restricted {
            let approval_id = format!("approval-{:06}", self.state.next_approval_id);
            self.state.next_approval_id += 1;
            self.state.approvals.insert(
                approval_id.clone(),
                ApprovalRecord {
                    approval_id: approval_id.clone(),
                    task_id: task_id.clone(),
                    state: "pending".to_string(),
                    reason: "policy require_approval".to_string(),
                },
            );
            task.current_step_summary = "awaiting approval for restricted capability".to_string();
            task.blocking_reason = Some(format!("approval_required approval_id={approval_id}"));
        }

        let mut trace = TraceSummary {
            trace_id: format!("trace-{}", task_id),
            task_id: task_id.clone(),
            events: vec![
                TraceEventSummary {
                    event_sequence: 1,
                    event_kind: "task_submitted".to_string(),
                    details: request.goal.clone(),
                },
                TraceEventSummary {
                    event_sequence: 2,
                    event_kind: "route_decision".to_string(),
                    details: "local_mock".to_string(),
                },
                TraceEventSummary {
                    event_sequence: 3,
                    event_kind: "verification_completed".to_string(),
                    details: "postconditions_satisfied".to_string(),
                },
            ],
        };

        let mut task_bindings: Vec<BindingRecord> = Vec::new();
        if !invalid_manifest {
            if restricted {
                let binding = self.new_binding(
                    &task_id,
                    &step_id,
                    BindingVisibility::ApprovalGated,
                    "approval-handle",
                    None,
                );
                push_trace_event(
                    &mut trace,
                    "binding_created",
                    &format!("binding_id={} visibility=approval_gated", binding.binding_id),
                );
                push_trace_event(
                    &mut trace,
                    "binding_redacted_for_model",
                    &format!("binding_id={}", binding.binding_id),
                );
                task_bindings.push(binding);
            } else {
                let binding = self.new_binding(
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
            let claim_entry = self.state.resource_claims.entry(resource_key.clone()).or_default();
            if !claim_entry.is_empty() {
                let conflict_id = format!("conflict-{:06}", self.state.next_conflict_id);
                self.state.next_conflict_id += 1;
                task.coordination_summary = Some(format!(
                    "conflict_detected conflict_id={} resource={}",
                    conflict_id, resource_key
                ));
                push_trace_event(
                    &mut trace,
                    "conflict_detected",
                    &format!("resource={} related_tasks={}", resource_key, claim_entry.join(",")),
                );
            }
            claim_entry.push(task_id.clone());
        }

        let mut artifacts = vec![
            ArtifactSummary {
                artifact_id: format!("artifact-{}-route", task_id),
                artifact_kind: "route_decision".to_string(),
                summary: "selected local mock route".to_string(),
            },
            ArtifactSummary {
                artifact_id: format!("artifact-{}-verification", task_id),
                artifact_kind: "verification_result".to_string(),
                summary: "postconditions satisfied".to_string(),
            },
        ];
        if task.task_state == "succeeded" {
            artifacts.push(ArtifactSummary {
                artifact_id: format!("artifact-{}-final", task_id),
                artifact_kind: "final_result".to_string(),
                summary: "task succeeded".to_string(),
            });
        }
        if invalid_manifest {
            artifacts.push(ArtifactSummary {
                artifact_id: format!("artifact-{}-manifest", task_id),
                artifact_kind: "failure_record".to_string(),
                summary: "capability manifest invalid".to_string(),
            });
        } else if restricted {
            artifacts.push(ArtifactSummary {
                artifact_id: format!("artifact-{}-approval", task_id),
                artifact_kind: "verification_result".to_string(),
                summary: "restricted step is approval gated".to_string(),
            });
        }
        if task.coordination_summary.is_some() {
            artifacts.push(ArtifactSummary {
                artifact_id: format!("artifact-{}-coordination", task_id),
                artifact_kind: "verification_result".to_string(),
                summary: "coordination summary recorded".to_string(),
            });
        }

        self.state.tasks.insert(task_id.clone(), task);
        self.state.traces.insert(task_id.clone(), trace);
        self.state.artifacts.insert(task_id.clone(), artifacts);
        self.state.bindings.insert(task_id.clone(), task_bindings);
        if let Some(idempotency_key) = namespaced_idempotency_key {
            self.state.idempotency_keys.insert(idempotency_key, task_id.clone());
        }

        let response_state = self
            .state
            .tasks
            .get(&task_id)
            .map(|t| t.task_state.clone())
            .unwrap_or_else(|| "succeeded".to_string());
        self.save()?;

        Ok(SubmitTaskOpResponse {
            task_id,
            task_state: response_state,
            summary: "task accepted".to_string(),
        })
    }

    pub fn get_task(&self, task_id: &str) -> Option<TaskSummary> {
        self.state.tasks.get(task_id).cloned()
    }

    pub fn get_trace(&self, task_id: &str) -> Option<TraceSummary> {
        self.state.traces.get(task_id).cloned()
    }

    pub fn get_artifacts(&self, task_id: &str) -> Vec<ArtifactSummary> {
        self.state.artifacts.get(task_id).cloned().unwrap_or_default()
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

        let (task_id, final_state, response_approval_id) = {
            let approval = self
                .state
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

        if let Some(task) = self.state.tasks.get_mut(&task_id) {
            if final_state == "approved" {
                task.task_state = "succeeded".to_string();
                task.current_step_summary = "restricted step approved and completed".to_string();
                task.blocking_reason = None;
            } else {
                task.task_state = "blocked".to_string();
                task.current_step_summary = "restricted step denied".to_string();
                task.blocking_reason = Some("approval_denied".to_string());
            }
        }

        if final_state == "approved" {
            let artifacts = self.state.artifacts.entry(task_id.clone()).or_default();
            let has_final = artifacts.iter().any(|a| a.artifact_kind == "final_result");
            if !has_final {
                artifacts.push(ArtifactSummary {
                    artifact_id: format!("artifact-{}-final", task_id),
                    artifact_kind: "final_result".to_string(),
                    summary: "task succeeded".to_string(),
                });
            }
        }

        if let Some(trace) = self.state.traces.get_mut(&task_id) {
            trace.events.push(TraceEventSummary {
                event_sequence: (trace.events.len() as u64) + 1,
                event_kind: "approval_resolved".to_string(),
                details: final_state.clone(),
            });
        }

        self.save()?;

        Ok(ResolveApprovalResponse {
            approval_id: response_approval_id,
            task_id,
            state: final_state,
        })
    }
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

impl Store {
    fn new_binding(
        &mut self,
        task_id: &str,
        step_id: &str,
        visibility: BindingVisibility,
        handle: &str,
        raw_value_model_text: Option<String>,
    ) -> BindingRecord {
        let binding_id = format!("binding-{:06}", self.state.next_binding_id);
        self.state.next_binding_id += 1;
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
}
