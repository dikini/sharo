use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sharo_core::coordination::{
    CoordinationChannelRecord, CoordinationClaimRecord, CoordinationConflictRecord, CoordinationIntentRecord,
};
use sharo_core::policy::{ActionClass, PolicyContext, PolicyDecisionKind, PolicyEngine};
use sharo_core::protocol::{
    ApprovalSummary, ArtifactSummary, ResolveApprovalResponse, SubmitTaskOpRequest,
    SubmitTaskOpResponse, TaskSummary, TraceEventSummary, TraceSummary,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionRecord {
    session_id: String,
    session_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApprovalRecord {
    approval_id: String,
    task_id: String,
    step_id: String,
    state: String,
    reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedState {
    sessions: BTreeMap<String, SessionRecord>,
    tasks: BTreeMap<String, TaskSummary>,
    traces: BTreeMap<String, TraceSummary>,
    artifacts: BTreeMap<String, Vec<ArtifactSummary>>,
    approvals: BTreeMap<String, ApprovalRecord>,
    coordination_intents: BTreeMap<String, CoordinationIntentRecord>,
    coordination_claims: BTreeMap<String, CoordinationClaimRecord>,
    coordination_conflicts: BTreeMap<String, CoordinationConflictRecord>,
    coordination_channels: BTreeMap<String, CoordinationChannelRecord>,
    next_session_id: u64,
    next_task_id: u64,
    next_approval_id: u64,
    next_intent_id: u64,
    next_claim_id: u64,
    next_conflict_id: u64,
    next_channel_id: u64,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            sessions: BTreeMap::new(),
            tasks: BTreeMap::new(),
            traces: BTreeMap::new(),
            artifacts: BTreeMap::new(),
            approvals: BTreeMap::new(),
            coordination_intents: BTreeMap::new(),
            coordination_claims: BTreeMap::new(),
            coordination_conflicts: BTreeMap::new(),
            coordination_channels: BTreeMap::new(),
            next_session_id: 1,
            next_task_id: 1,
            next_approval_id: 1,
            next_intent_id: 1,
            next_claim_id: 1,
            next_conflict_id: 1,
            next_channel_id: 1,
        }
    }
}

pub struct Store {
    path: PathBuf,
    state: PersistedState,
    policy_engine: PolicyEngine,
}

impl Store {
    fn extract_scope(goal: &str) -> String {
        for token in goal.split_whitespace() {
            if let Some(rest) = token.strip_prefix("scope:") {
                if !rest.is_empty() {
                    return rest.to_string();
                }
            }
        }
        "general".to_string()
    }

    fn find_overlap_task(&self, task_id: &str, scope: &str) -> Option<String> {
        self.state
            .coordination_claims
            .values()
            .find(|claim| claim.scope == scope && claim.task_id != task_id && claim.state == "active")
            .map(|claim| claim.task_id.clone())
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref().to_path_buf();
        if !path.exists() {
            return Ok(Self {
                path,
                state: PersistedState::default(),
                policy_engine: PolicyEngine,
            });
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| format!("store_read_failed path={} error={}", path.display(), e))?;

        let state = serde_json::from_str::<PersistedState>(&content)
            .map_err(|e| format!("store_parse_failed path={} error={}", path.display(), e))?;

        Ok(Self {
            path,
            state,
            policy_engine: PolicyEngine,
        })
    }

    fn save(&self) -> Result<(), String> {
        let data = serde_json::to_string_pretty(&self.state)
            .map_err(|e| format!("store_serialize_failed error={}", e))?;
        fs::write(&self.path, data)
            .map_err(|e| format!("store_write_failed path={} error={}", self.path.display(), e))
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
            .unwrap_or_else(|| "session-implicit".to_string());

        let task_id = format!("task-{:06}", self.state.next_task_id);
        self.state.next_task_id += 1;

        let step_id = format!("step-{}-001", task_id);
        let scope = Self::extract_scope(&request.goal);

        let intent_id = format!("intent-{:06}", self.state.next_intent_id);
        self.state.next_intent_id += 1;
        self.state.coordination_intents.insert(
            intent_id.clone(),
            CoordinationIntentRecord {
                intent_id,
                task_id: task_id.clone(),
                scope: scope.clone(),
                goal: request.goal.clone(),
            },
        );

        let claim_id = format!("claim-{:06}", self.state.next_claim_id);
        self.state.next_claim_id += 1;
        self.state.coordination_claims.insert(
            claim_id.clone(),
            CoordinationClaimRecord {
                claim_id,
                task_id: task_id.clone(),
                scope: scope.clone(),
                state: "active".to_string(),
            },
        );

        let overlap_task_id = self.find_overlap_task(&task_id, &scope);

        let action_class = if request.goal.contains("write") || request.goal.contains("draft") {
            ActionClass::RestrictedWrite
        } else {
            ActionClass::Read
        };

        let decision = self.policy_engine.evaluate(&PolicyContext {
            task_id: task_id.clone(),
            step_id: step_id.clone(),
            action_class,
            autonomy_mode: "supervised".to_string(),
        });

        let mut events = vec![
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
                event_kind: "policy_decision".to_string(),
                details: decision.reason.clone(),
            },
        ];

        let mut artifacts = vec![ArtifactSummary {
            artifact_id: format!("artifact-{}-route", task_id),
            artifact_kind: "route_decision".to_string(),
            summary: "selected local mock route".to_string(),
        }];
        let mut coordination_summary = None;

        if let Some(related_task_id) = overlap_task_id {
            let conflict_id = format!("conflict-{:06}", self.state.next_conflict_id);
            self.state.next_conflict_id += 1;
            self.state.coordination_conflicts.insert(
                conflict_id.clone(),
                CoordinationConflictRecord {
                    conflict_id: conflict_id.clone(),
                    task_id: task_id.clone(),
                    related_task_id: related_task_id.clone(),
                    scope: scope.clone(),
                },
            );

            let channel_id = format!("channel-{:06}", self.state.next_channel_id);
            self.state.next_channel_id += 1;
            self.state.coordination_channels.insert(
                channel_id.clone(),
                CoordinationChannelRecord {
                    channel_id,
                    conflict_id: conflict_id.clone(),
                    task_id: task_id.clone(),
                    related_task_id: related_task_id.clone(),
                },
            );

            events.push(TraceEventSummary {
                event_sequence: 4,
                event_kind: "coordination_conflict".to_string(),
                details: format!(
                    "conflict_id={} scope={} related_task_id={}",
                    conflict_id, scope, related_task_id
                ),
            });
            coordination_summary = Some(format!(
                "conflict_id={} scope={} related_task_id={}",
                conflict_id, scope, related_task_id
            ));
        }

        let (task_state, current_step_summary, blocking_reason) = match decision.decision {
            PolicyDecisionKind::Allow => {
                events.push(TraceEventSummary {
                    event_sequence: events.len() as u64 + 1,
                    event_kind: "verification_completed".to_string(),
                    details: "postconditions_satisfied".to_string(),
                });

                artifacts.push(ArtifactSummary {
                    artifact_id: format!("artifact-{}-verification", task_id),
                    artifact_kind: "verification_result".to_string(),
                    summary: "postconditions satisfied".to_string(),
                });

                artifacts.push(ArtifactSummary {
                    artifact_id: format!("artifact-{}-final", task_id),
                    artifact_kind: "final_result".to_string(),
                    summary: "task succeeded".to_string(),
                });

                ("succeeded".to_string(), "read one context item".to_string(), None)
            }
            PolicyDecisionKind::RequireApproval => {
                let approval_id = format!("approval-{:06}", self.state.next_approval_id);
                self.state.next_approval_id += 1;

                self.state.approvals.insert(
                    approval_id.clone(),
                    ApprovalRecord {
                        approval_id: approval_id.clone(),
                        task_id: task_id.clone(),
                        step_id: step_id.clone(),
                        state: "pending".to_string(),
                        reason: decision.reason.clone(),
                    },
                );

                events.push(TraceEventSummary {
                    event_sequence: events.len() as u64 + 1,
                    event_kind: "approval_requested".to_string(),
                    details: approval_id.clone(),
                });

                (
                    "awaiting_approval".to_string(),
                    "restricted write pending approval".to_string(),
                    Some(format!("approval_required approval_id={}", approval_id)),
                )
            }
            PolicyDecisionKind::Deny => {
                events.push(TraceEventSummary {
                    event_sequence: events.len() as u64 + 1,
                    event_kind: "policy_denied".to_string(),
                    details: decision.reason.clone(),
                });
                (
                    "blocked".to_string(),
                    "restricted write denied".to_string(),
                    Some(decision.reason.clone()),
                )
            }
        };

        let task = TaskSummary {
            task_id: task_id.clone(),
            session_id: session_id.clone(),
            task_state: task_state.clone(),
            current_step_summary,
            blocking_reason,
            coordination_summary,
        };

        let trace = TraceSummary {
            trace_id: format!("trace-{}", task_id),
            task_id: task_id.clone(),
            events,
        };

        self.state.tasks.insert(task_id.clone(), task);
        self.state.traces.insert(task_id.clone(), trace);
        self.state.artifacts.insert(task_id.clone(), artifacts);

        self.save()?;

        Ok(SubmitTaskOpResponse {
            task_id,
            task_state,
            summary: "task admitted".to_string(),
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

    pub fn list_pending_approvals(&self) -> Vec<ApprovalSummary> {
        self.state
            .approvals
            .values()
            .filter(|a| a.state == "pending")
            .map(|a| ApprovalSummary {
                approval_id: a.approval_id.clone(),
                task_id: a.task_id.clone(),
                step_id: a.step_id.clone(),
                state: a.state.clone(),
                reason: a.reason.clone(),
            })
            .collect()
    }

    pub fn resolve_approval(&mut self, approval_id: &str, decision: &str) -> Result<ResolveApprovalResponse, String> {
        let approval = self
            .state
            .approvals
            .get_mut(approval_id)
            .ok_or_else(|| format!("approval_not_found approval_id={}", approval_id))?;

        if approval.state != "pending" {
            return Ok(ResolveApprovalResponse {
                approval_id: approval.approval_id.clone(),
                task_id: approval.task_id.clone(),
                state: approval.state.clone(),
                summary: "idempotent_replay".to_string(),
            });
        }

        let task = self
            .state
            .tasks
            .get_mut(&approval.task_id)
            .ok_or_else(|| format!("task_not_found task_id={}", approval.task_id))?;

        let trace = self
            .state
            .traces
            .get_mut(&approval.task_id)
            .ok_or_else(|| format!("trace_not_found task_id={}", approval.task_id))?;

        let next_seq = trace.events.last().map(|e| e.event_sequence + 1).unwrap_or(1);

        match decision {
            "approve" => {
                approval.state = "approved".to_string();
                task.task_state = "succeeded".to_string();
                task.current_step_summary = "restricted write approved and completed".to_string();
                task.blocking_reason = None;

                trace.events.push(TraceEventSummary {
                    event_sequence: next_seq,
                    event_kind: "approval_resolved".to_string(),
                    details: "approved".to_string(),
                });
                trace.events.push(TraceEventSummary {
                    event_sequence: next_seq + 1,
                    event_kind: "verification_completed".to_string(),
                    details: "postconditions_satisfied".to_string(),
                });

                let artifacts = self
                    .state
                    .artifacts
                    .entry(approval.task_id.clone())
                    .or_default();
                artifacts.push(ArtifactSummary {
                    artifact_id: format!("artifact-{}-verification", approval.task_id),
                    artifact_kind: "verification_result".to_string(),
                    summary: "restricted action verified".to_string(),
                });
                artifacts.push(ArtifactSummary {
                    artifact_id: format!("artifact-{}-final", approval.task_id),
                    artifact_kind: "final_result".to_string(),
                    summary: "task succeeded".to_string(),
                });
            }
            "deny" => {
                approval.state = "denied".to_string();
                task.task_state = "blocked".to_string();
                task.current_step_summary = "restricted write denied".to_string();
                task.blocking_reason = Some("approval_denied".to_string());

                trace.events.push(TraceEventSummary {
                    event_sequence: next_seq,
                    event_kind: "approval_resolved".to_string(),
                    details: "denied".to_string(),
                });
            }
            other => {
                return Err(format!("invalid_approval_decision decision={}", other));
            }
        }

        let approval_id = approval.approval_id.clone();
        let task_id = approval.task_id.clone();
        let state = approval.state.clone();
        self.save()?;

        Ok(ResolveApprovalResponse {
            approval_id,
            task_id,
            state,
            summary: "approval_resolved".to_string(),
        })
    }
}
