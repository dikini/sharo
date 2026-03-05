use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskState {
    Submitted,
    Running,
    Succeeded,
    Failed,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmitTaskRequest {
    pub session_id: Option<String>,
    pub goal: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmitTaskResponse {
    pub task_id: String,
    pub state: TaskState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskStatusRequest {
    pub task_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskStatusResponse {
    pub task_id: String,
    pub state: TaskState,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisterSessionRequest {
    pub session_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisterSessionResponse {
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmitTaskOpRequest {
    pub session_id: Option<String>,
    pub goal: String,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmitTaskOpResponse {
    pub task_id: String,
    pub task_state: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetTaskRequest {
    pub task_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskSummary {
    pub task_id: String,
    pub session_id: String,
    pub task_state: String,
    pub current_step_summary: String,
    pub blocking_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetTaskResponse {
    pub task: TaskSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetTraceRequest {
    pub task_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceEventSummary {
    pub event_sequence: u64,
    pub event_kind: String,
    pub details: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceSummary {
    pub trace_id: String,
    pub task_id: String,
    pub events: Vec<TraceEventSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetTraceResponse {
    pub trace: TraceSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetArtifactsRequest {
    pub task_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactSummary {
    pub artifact_id: String,
    pub artifact_kind: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetArtifactsResponse {
    pub artifacts: Vec<ArtifactSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DaemonRequest {
    Submit(SubmitTaskRequest),
    Status(TaskStatusRequest),
    RegisterSession(RegisterSessionRequest),
    SubmitTask(SubmitTaskOpRequest),
    GetTask(GetTaskRequest),
    GetTrace(GetTraceRequest),
    GetArtifacts(GetArtifactsRequest),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DaemonResponse {
    Submit(SubmitTaskResponse),
    Status(TaskStatusResponse),
    RegisterSession(RegisterSessionResponse),
    SubmitTask(SubmitTaskOpResponse),
    GetTask(GetTaskResponse),
    GetTrace(GetTraceResponse),
    GetArtifacts(GetArtifactsResponse),
    Error { message: String },
}
