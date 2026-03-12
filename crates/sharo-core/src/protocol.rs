use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::mcp::{McpServerSummary, RuntimeStatusSummary};
use crate::skills::{SkillCatalogEntry, SkillDocument};

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

pub fn object_schema_well_formed(schema: &ObjectSchema) -> bool {
    if schema.allow_additional {
        return true;
    }
    schema.allowed.is_superset(&schema.required)
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
    if !object_schema_well_formed(expected) || !object_schema_well_formed(tool) {
        return false;
    }
    if !expected.required.is_superset(&tool.required) {
        return false;
    }
    if !expected.allow_additional && tool.allow_additional {
        return false;
    }
    if !tool.allow_additional && !tool.allowed.is_superset(&expected.allowed) {
        return false;
    }
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HookSchemaDescriptor {
    pub input: ObjectSchema,
    pub output: ObjectSchema,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecollectionLintLimits {
    pub max_cards: usize,
    pub max_payload_bytes: usize,
    pub max_tokens: usize,
}

impl Default for RecollectionLintLimits {
    fn default() -> Self {
        Self {
            max_cards: 32,
            max_payload_bytes: 65_536,
            max_tokens: 4096,
        }
    }
}

pub fn estimate_policy_id_tokens(policy_ids: &[String]) -> usize {
    policy_ids
        .iter()
        .flat_map(|value| value.split_whitespace())
        .count()
}

pub fn estimate_recollection_card_tokens(card: &RecollectionCard) -> usize {
    card.subject.split_whitespace().count()
        + card.text.split_whitespace().count()
        + card
            .policy_ids
            .iter()
            .flat_map(|value| value.split_whitespace())
            .count()
        + card
            .provenance
            .iter()
            .map(|provenance| {
                provenance.source_ref.split_whitespace().count()
                    + provenance
                        .source_excerpt
                        .as_deref()
                        .map_or(0, |value| value.split_whitespace().count())
            })
            .sum::<usize>()
}

pub fn estimate_recollection_tokens(payload: &RecollectionPayload) -> usize {
    let policy_tokens = estimate_policy_id_tokens(&payload.policy_ids);
    let card_tokens = payload
        .cards
        .iter()
        .map(estimate_recollection_card_tokens)
        .sum::<usize>();
    policy_tokens + card_tokens
}

pub fn output_schema_compatible(expected: &ObjectSchema, tool: &ObjectSchema) -> bool {
    if !object_schema_well_formed(expected) || !object_schema_well_formed(tool) {
        return false;
    }
    if !tool.required.is_superset(&expected.required) {
        return false;
    }
    if !expected.allow_additional && tool.allow_additional {
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
    validate_recollection_payload_with_limits(value, &RecollectionLintLimits::default())
}

pub fn validate_recollection_payload_with_limits(
    value: &Value,
    limits: &RecollectionLintLimits,
) -> Result<RecollectionPayload, String> {
    let payload: RecollectionPayload = serde_json::from_value(value.clone())
        .map_err(|error| format!("pre_prompt_output_schema_invalid error={error}"))?;
    semantic_lint_recollection_payload_with_limits(&payload, limits)?;
    Ok(payload)
}

pub fn semantic_lint_recollection_payload(payload: &RecollectionPayload) -> Result<(), String> {
    semantic_lint_recollection_payload_with_limits(payload, &RecollectionLintLimits::default())
}

pub fn semantic_lint_recollection_payload_with_limits(
    payload: &RecollectionPayload,
    limits: &RecollectionLintLimits,
) -> Result<(), String> {
    if payload.cards.is_empty() {
        return Err("pre_prompt_output_semantic_invalid reason=cards_empty".to_string());
    }
    if payload.cards.len() > limits.max_cards {
        return Err(format!(
            "pre_prompt_output_semantic_invalid reason=max_cards_exceeded actual={} max={}",
            payload.cards.len(),
            limits.max_cards
        ));
    }
    let payload_bytes = serde_json::to_vec(payload)
        .map_err(|error| {
            format!("pre_prompt_output_semantic_invalid reason=encode_failed error={error}")
        })?
        .len();
    if payload_bytes > limits.max_payload_bytes {
        return Err(format!(
            "pre_prompt_output_semantic_invalid reason=max_payload_bytes_exceeded actual={} max={}",
            payload_bytes, limits.max_payload_bytes
        ));
    }
    let estimated_tokens = estimate_recollection_tokens(payload);
    if estimated_tokens > limits.max_tokens {
        return Err(format!(
            "pre_prompt_output_semantic_invalid reason=max_tokens_exceeded actual={} max={}",
            estimated_tokens, limits.max_tokens
        ));
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
pub struct SessionSummary {
    pub session_id: String,
    pub session_label: String,
    pub session_status: String,
    pub activity_sequence: u64,
    pub latest_task_id: Option<String>,
    pub latest_task_state: Option<String>,
    pub latest_result_preview: Option<String>,
    pub has_pending_approval: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListSessionsResponse {
    pub sessions: Vec<SessionSummary>,
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
pub struct GetSessionTasksRequest {
    pub session_id: String,
    pub task_limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetSessionTasksResponse {
    pub tasks: Vec<TaskSummary>,
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
pub struct GetSessionViewRequest {
    pub session_id: String,
    pub task_limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionView {
    pub session_id: String,
    pub session_label: String,
    pub tasks: Vec<TaskSummary>,
    pub pending_approvals: Vec<ApprovalSummary>,
    pub latest_result_preview: Option<String>,
    pub active_blocking_task_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetSessionViewResponse {
    pub session: SessionView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListSkillsRequest {
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListSkillsResponse {
    pub skills: Vec<SkillCatalogEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetSkillRequest {
    pub skill_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetSkillResponse {
    pub skill: SkillDocument,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetSessionSkillsRequest {
    pub session_id: String,
    pub active_skill_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetSessionSkillsResponse {
    pub session_id: String,
    pub active_skill_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListMcpServersResponse {
    pub servers: Vec<McpServerSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateMcpServerStateRequest {
    pub server_id: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateMcpServerStateResponse {
    pub server: McpServerSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetRuntimeStatusResponse {
    pub status: RuntimeStatusSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HazelActionAvailability {
    pub retrieval_preview: bool,
    pub validate_batch: bool,
    pub submit_batch: bool,
    pub enqueue_sleep_job: bool,
    pub cancel_sleep_job: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HazelLimitsSummary {
    pub max_list_items: u32,
    pub max_preview_cards: u32,
    pub max_sleep_batches: u32,
    pub max_sleep_proposals_per_batch: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HazelStatusSummary {
    pub available: bool,
    pub card_count: u32,
    pub proposal_batch_count: u32,
    pub sleep_job_count: u32,
    pub actions: HazelActionAvailability,
    pub limits: HazelLimitsSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetHazelStatusResponse {
    pub status: HazelStatusSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListHazelCardsRequest {
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HazelCardView {
    pub card_id: String,
    pub kind: RecollectionCardKind,
    pub state: RecollectionCardState,
    pub subject: String,
    pub text: String,
    pub provenance: Vec<ProvenanceRef>,
    pub policy_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListHazelCardsResponse {
    pub cards: Vec<HazelCardView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetHazelCardRequest {
    pub card_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetHazelCardResponse {
    pub card: HazelCardView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListHazelProposalBatchesRequest {
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HazelProposalBatchView {
    pub batch_id: String,
    pub idempotency_key: String,
    pub source_ref: String,
    pub producer: String,
    pub proposal_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListHazelProposalBatchesResponse {
    pub batches: Vec<HazelProposalBatchView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetHazelProposalBatchRequest {
    pub batch_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetHazelProposalBatchResponse {
    pub batch: HazelProposalBatchView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HazelSleepJobState {
    Pending,
    Completed,
    Failed,
    Canceled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HazelSleepJobView {
    pub job_id: String,
    pub state: HazelSleepJobState,
    pub run_id: Option<String>,
    pub proposal_batch_ids: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListHazelSleepJobsRequest {
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListHazelSleepJobsResponse {
    pub jobs: Vec<HazelSleepJobView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetHazelSleepJobRequest {
    pub job_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetHazelSleepJobResponse {
    pub job: HazelSleepJobView,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HazelRetrievalPreviewRequest {
    pub input: PrePromptComposeHookInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HazelRetrievalPreviewResponse {
    pub preview_id: String,
    pub payload: RecollectionPayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidateHazelProposalBatchRequest {
    pub batch_id: String,
    #[serde(default)]
    pub strict_policy_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidateHazelProposalBatchResponse {
    pub validation_id: String,
    pub batch_id: String,
    pub accepted: bool,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmitHazelProposalBatchRequest {
    pub batch_id: String,
    #[serde(default)]
    pub strict_policy_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmitHazelProposalBatchResponse {
    pub submission_id: String,
    pub batch_id: String,
    pub state: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HazelConversationMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnqueueHazelSleepJobRequest {
    pub job_id: Option<String>,
    pub source_ref: String,
    pub idempotency_key: String,
    pub messages: Vec<HazelConversationMessage>,
    pub max_batches: u32,
    pub max_proposals_per_batch: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnqueueHazelSleepJobResponse {
    pub job: HazelSleepJobView,
    pub proposal_batch_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CancelHazelSleepJobRequest {
    pub job_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CancelHazelSleepJobResponse {
    pub job: HazelSleepJobView,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DaemonRequest {
    Submit(SubmitTaskRequest),
    Status(TaskStatusRequest),
    RegisterSession(RegisterSessionRequest),
    ListSessions,
    SubmitTask(SubmitTaskOpRequest),
    GetTask(GetTaskRequest),
    GetSessionTasks(GetSessionTasksRequest),
    GetSessionView(GetSessionViewRequest),
    ListSkills(ListSkillsRequest),
    GetSkill(GetSkillRequest),
    SetSessionSkills(SetSessionSkillsRequest),
    ListMcpServers,
    UpdateMcpServerState(UpdateMcpServerStateRequest),
    GetRuntimeStatus,
    GetHazelStatus,
    ListHazelCards(ListHazelCardsRequest),
    GetHazelCard(GetHazelCardRequest),
    ListHazelProposalBatches(ListHazelProposalBatchesRequest),
    GetHazelProposalBatch(GetHazelProposalBatchRequest),
    ListHazelSleepJobs(ListHazelSleepJobsRequest),
    GetHazelSleepJob(GetHazelSleepJobRequest),
    HazelPreview(HazelRetrievalPreviewRequest),
    ValidateHazelProposalBatch(ValidateHazelProposalBatchRequest),
    SubmitHazelProposalBatch(SubmitHazelProposalBatchRequest),
    EnqueueHazelSleepJob(EnqueueHazelSleepJobRequest),
    CancelHazelSleepJob(CancelHazelSleepJobRequest),
    GetTrace(GetTraceRequest),
    GetArtifacts(GetArtifactsRequest),
    ListPendingApprovals,
    ResolveApproval(ResolveApprovalRequest),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DaemonResponse {
    Submit(SubmitTaskResponse),
    Status(TaskStatusResponse),
    RegisterSession(RegisterSessionResponse),
    ListSessions(ListSessionsResponse),
    SubmitTask(SubmitTaskOpResponse),
    GetTask(GetTaskResponse),
    GetSessionTasks(GetSessionTasksResponse),
    GetSessionView(GetSessionViewResponse),
    ListSkills(ListSkillsResponse),
    GetSkill(GetSkillResponse),
    SetSessionSkills(SetSessionSkillsResponse),
    ListMcpServers(ListMcpServersResponse),
    UpdateMcpServerState(UpdateMcpServerStateResponse),
    GetRuntimeStatus(GetRuntimeStatusResponse),
    GetHazelStatus(GetHazelStatusResponse),
    ListHazelCards(ListHazelCardsResponse),
    GetHazelCard(GetHazelCardResponse),
    ListHazelProposalBatches(ListHazelProposalBatchesResponse),
    GetHazelProposalBatch(GetHazelProposalBatchResponse),
    ListHazelSleepJobs(ListHazelSleepJobsResponse),
    GetHazelSleepJob(GetHazelSleepJobResponse),
    HazelPreview(HazelRetrievalPreviewResponse),
    ValidateHazelProposalBatch(ValidateHazelProposalBatchResponse),
    SubmitHazelProposalBatch(SubmitHazelProposalBatchResponse),
    EnqueueHazelSleepJob(EnqueueHazelSleepJobResponse),
    CancelHazelSleepJob(CancelHazelSleepJobResponse),
    GetTrace(GetTraceResponse),
    GetArtifacts(GetArtifactsResponse),
    ListPendingApprovals(ListPendingApprovalsResponse),
    ResolveApproval(ResolveApprovalResponse),
    Error { message: String },
}
