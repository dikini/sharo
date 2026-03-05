use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sharo_core::protocol::{
    ArtifactSummary, SubmitTaskOpRequest, SubmitTaskOpResponse, TaskSummary, TraceEventSummary,
    TraceSummary,
};

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
    next_session_id: u64,
    next_task_id: u64,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            sessions: BTreeMap::new(),
            tasks: BTreeMap::new(),
            traces: BTreeMap::new(),
            artifacts: BTreeMap::new(),
            next_session_id: 1,
            next_task_id: 1,
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

        let task = TaskSummary {
            task_id: task_id.clone(),
            session_id: session_id.clone(),
            task_state: "succeeded".to_string(),
            current_step_summary: "read one context item".to_string(),
            blocking_reason: None,
        };

        let trace = TraceSummary {
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

        let artifacts = vec![
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
            ArtifactSummary {
                artifact_id: format!("artifact-{}-final", task_id),
                artifact_kind: "final_result".to_string(),
                summary: "task succeeded".to_string(),
            },
        ];

        self.state.tasks.insert(task_id.clone(), task);
        self.state.traces.insert(task_id.clone(), trace);
        self.state.artifacts.insert(task_id.clone(), artifacts);

        self.save()?;

        Ok(SubmitTaskOpResponse {
            task_id,
            task_state: "succeeded".to_string(),
            summary: "read path executed and verified".to_string(),
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
}
