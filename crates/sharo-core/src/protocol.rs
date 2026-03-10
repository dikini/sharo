use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecollectionCardKind {
    SoftRecollection,
    StrongConstraint,
    SupportingContext,
    AssociationCue,
    DoNotUse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecollectionCardState {
    Candidate,
    Active,
    Contested,
    Deprecated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvenanceRef {
    pub source_ref: String,
    pub source_excerpt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecollectionCard {
    pub card_id: String,
    pub kind: RecollectionCardKind,
    pub state: RecollectionCardState,
    pub subject: String,
    pub text: String,
    pub provenance: Vec<ProvenanceRef>,
    pub policy_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecollectionPayload {
    pub policy_ids: Vec<String>,
    pub cards: Vec<RecollectionCard>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HazelCardPolicyHint {
    pub kind: RecollectionCardKind,
    pub policy_ids: Vec<String>,
    pub max_cards: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyMergeMode {
    StrictestWins,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyRule {
    LabelGuesses,
    PreferSupportedFacts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectivePolicyBundle {
    pub effective_policy_ids: Vec<String>,
    pub merge_mode: PolicyMergeMode,
    pub rules: Vec<PolicyRule>,
}

impl EffectivePolicyBundle {
    pub fn new(
        mut effective_policy_ids: Vec<String>,
        merge_mode: PolicyMergeMode,
        rules: Vec<PolicyRule>,
    ) -> Self {
        effective_policy_ids.sort();
        effective_policy_ids.dedup();
        Self {
            effective_policy_ids,
            merge_mode,
            rules,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrePromptComposeHookInput {
    pub session_id: String,
    pub task_id: String,
    pub goal: String,
    pub runtime: String,
    pub top_k: Option<usize>,
    pub token_budget: Option<usize>,
    pub relevance_threshold: Option<f32>,
    #[serde(default)]
    pub policy_ids: Vec<String>,
    #[serde(default)]
    pub card_policy_hints: Vec<HazelCardPolicyHint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolCallRequest {
    pub tool: String,
    pub input: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolCallResponse {
    pub ok: bool,
    pub output: Option<Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectSchema {
    pub required: BTreeSet<String>,
    pub allowed: BTreeSet<String>,
    pub allow_additional: bool,
}

impl ObjectSchema {
    pub fn new(required: &[&str], allowed: &[&str], allow_additional: bool) -> Self {
        Self {
            required: required.iter().map(|value| (*value).to_string()).collect(),
            allowed: allowed.iter().map(|value| (*value).to_string()).collect(),
            allow_additional,
        }
    }
}

pub fn expected_pre_prompt_compose_input_schema() -> ObjectSchema {
    ObjectSchema::new(
        &[
            "session_id",
            "task_id",
            "goal",
            "runtime",
            "top_k",
            "token_budget",
            "relevance_threshold",
            "policy_ids",
            "card_policy_hints",
        ],
        &[
            "session_id",
            "task_id",
            "goal",
            "runtime",
            "top_k",
            "token_budget",
            "relevance_threshold",
            "policy_ids",
            "card_policy_hints",
        ],
        false,
    )
}

pub fn expected_recollection_output_schema() -> ObjectSchema {
    ObjectSchema::new(&["policy_ids", "cards"], &["policy_ids", "cards"], false)
}

pub fn input_schema_compatible(expected: &ObjectSchema, tool: &ObjectSchema) -> bool {
    if !expected.required.is_superset(&tool.required) {
        return false;
    }
    if !tool.allow_additional && !tool.allowed.is_superset(&expected.allowed) {
        return false;
    }
    true
}

pub fn output_schema_compatible(expected: &ObjectSchema, tool: &ObjectSchema) -> bool {
    if !tool.required.is_superset(&expected.required) {
        return false;
    }
    if !expected.allow_additional && !expected.allowed.is_superset(&tool.allowed) {
        return false;
    }
    true
}

pub fn validate_pre_prompt_compose_input_value(
    value: &Value,
) -> Result<PrePromptComposeHookInput, String> {
    serde_json::from_value(value.clone())
        .map_err(|error| format!("pre_prompt_input_schema_invalid error={error}"))
}

pub fn validate_recollection_payload_value(value: &Value) -> Result<RecollectionPayload, String> {
    let payload: RecollectionPayload = serde_json::from_value(value.clone())
        .map_err(|error| format!("pre_prompt_output_schema_invalid error={error}"))?;
    semantic_lint_recollection_payload(&payload)?;
    Ok(payload)
}

pub fn semantic_lint_recollection_payload(payload: &RecollectionPayload) -> Result<(), String> {
    if payload.cards.is_empty() {
        return Err("pre_prompt_output_semantic_invalid reason=cards_empty".to_string());
    }
    for card in &payload.cards {
        if card.provenance.is_empty() {
            return Err(format!(
                "pre_prompt_output_semantic_invalid reason=missing_provenance card_id={}",
                card.card_id
            ));
        }
    }
    Ok(())
}

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
    pub coordination_summary: Option<String>,
    pub result_preview: Option<String>,
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
    pub session_id: String,
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
    pub produced_by_step_id: String,
    pub produced_by_trace_event_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetArtifactsResponse {
    pub artifacts: Vec<ArtifactSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListPendingApprovalsResponse {
    pub approvals: Vec<ApprovalSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalSummary {
    pub approval_id: String,
    pub task_id: String,
    pub state: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolveApprovalRequest {
    pub approval_id: String,
    pub decision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolveApprovalResponse {
    pub approval_id: String,
    pub task_id: String,
    pub state: String,
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
    ListPendingApprovals,
    ResolveApproval(ResolveApprovalRequest),
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
    ListPendingApprovals(ListPendingApprovalsResponse),
    ResolveApproval(ResolveApprovalResponse),
    Error { message: String },
}
