use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStateV2 {
    Submitted,
    Queued,
    Running,
    AwaitingApproval,
    Blocked,
    Succeeded,
    Failed,
    Cancelled,
}

impl TaskStateV2 {
    pub fn can_transition_to(self, next: TaskStateV2) -> bool {
        match (self, next) {
            (TaskStateV2::Submitted, TaskStateV2::Queued)
            | (TaskStateV2::Submitted, TaskStateV2::Failed)
            | (TaskStateV2::Queued, TaskStateV2::Running)
            | (TaskStateV2::Queued, TaskStateV2::Cancelled)
            | (TaskStateV2::Queued, TaskStateV2::Failed)
            | (TaskStateV2::Running, TaskStateV2::AwaitingApproval)
            | (TaskStateV2::Running, TaskStateV2::Blocked)
            | (TaskStateV2::Running, TaskStateV2::Succeeded)
            | (TaskStateV2::Running, TaskStateV2::Failed)
            | (TaskStateV2::Running, TaskStateV2::Cancelled)
            | (TaskStateV2::AwaitingApproval, TaskStateV2::Running)
            | (TaskStateV2::AwaitingApproval, TaskStateV2::Blocked)
            | (TaskStateV2::AwaitingApproval, TaskStateV2::Failed)
            | (TaskStateV2::Blocked, TaskStateV2::Cancelled)
            | (TaskStateV2::Blocked, TaskStateV2::Failed) => true,
            _ if self == next => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepState {
    Proposed,
    Ready,
    Executing,
    AwaitingApproval,
    Blocked,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactKind {
    RouteDecision,
    CapabilityResult,
    VerificationResult,
    FailureRecord,
    FinalResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskRecord {
    pub task_id: String,
    pub session_id: String,
    pub goal: String,
    pub task_state: TaskStateV2,
    pub current_step_id: Option<String>,
    pub result_artifact_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepRecord {
    pub step_id: String,
    pub task_id: String,
    pub step_state: StepState,
    pub summary: String,
    pub requested_capability: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRecord {
    pub artifact_id: String,
    pub task_id: String,
    pub artifact_kind: ArtifactKind,
    pub summary: String,
    pub content: String,
    pub produced_by_step_id: String,
    pub produced_by_trace_event_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceEvent {
    pub event_sequence: u64,
    pub event_kind: String,
    pub details: String,
}

impl TraceEvent {
    pub fn new(event_sequence: u64, event_kind: &str, details: &str) -> Self {
        Self {
            event_sequence,
            event_kind: event_kind.to_string(),
            details: details.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceRecord {
    pub trace_id: String,
    pub task_id: String,
    pub session_id: String,
    pub events: Vec<TraceEvent>,
}

impl TraceRecord {
    pub fn new(trace_id: &str, task_id: &str, session_id: &str) -> Self {
        Self {
            trace_id: trace_id.to_string(),
            task_id: task_id.to_string(),
            session_id: session_id.to_string(),
            events: Vec::new(),
        }
    }

    pub fn push_event(&mut self, event: TraceEvent) {
        self.events.push(event);
    }

    pub fn is_monotonic(&self) -> bool {
        self.events
            .windows(2)
            .all(|pair| pair[0].event_sequence < pair[1].event_sequence)
    }
}
