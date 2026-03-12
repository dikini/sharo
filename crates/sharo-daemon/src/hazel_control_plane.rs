use std::collections::BTreeSet;

use sharo_hazel_core::ingest::{
    OpenAiMessage, import_openai_messages_to_proposal_batch, submit_proposal_batch,
    validate_proposal_batch,
};
use sharo_hazel_core::proposal::ProposalBatch;
use sharo_hazel_core::retrieval::HazelMemoryCore;
use sharo_hazel_core::sleep::{SleepBudget, SleepJobOutput, derive_sleep_run_id, validate_sleep_budget, validate_sleep_output};
use sharo_core::protocol::{
    CancelHazelSleepJobResponse, EnqueueHazelSleepJobRequest, EnqueueHazelSleepJobResponse,
    GetHazelStatusResponse, HazelActionAvailability, HazelCardView, HazelConversationMessage,
    HazelLimitsSummary, HazelProposalBatchView, HazelRetrievalPreviewRequest,
    HazelRetrievalPreviewResponse, HazelSleepJobState, HazelSleepJobView, HazelStatusSummary,
    ListHazelCardsResponse, ListHazelProposalBatchesResponse, ListHazelSleepJobsResponse,
    RecollectionCard, SubmitHazelProposalBatchRequest, SubmitHazelProposalBatchResponse,
    ValidateHazelProposalBatchRequest, ValidateHazelProposalBatchResponse,
};

use crate::store::Store;

const MAX_LIST_ITEMS: usize = 64;
const MAX_PREVIEW_CARDS: u32 = 8;
const MAX_PREVIEW_TOKEN_BUDGET: usize = 65_536;
const MAX_SLEEP_BATCHES: u32 = 8;
const MAX_SLEEP_PROPOSALS_PER_BATCH: u32 = 64;
const KNOWN_POLICY_IDS: &[&str] = &["hunch.v1", "inspection.default"];

pub fn get_hazel_status(store: &Store) -> GetHazelStatusResponse {
    let card_count = inspect_hazel_cards().len().min(MAX_LIST_ITEMS) as u32;
    GetHazelStatusResponse {
        status: HazelStatusSummary {
            available: true,
            card_count,
            proposal_batch_count: store.hazel_proposal_batch_count().min(MAX_LIST_ITEMS) as u32,
            sleep_job_count: store.hazel_sleep_job_count().min(MAX_LIST_ITEMS) as u32,
            actions: HazelActionAvailability {
                retrieval_preview: true,
                validate_batch: true,
                submit_batch: true,
                enqueue_sleep_job: true,
                cancel_sleep_job: true,
            },
            limits: HazelLimitsSummary {
                max_list_items: MAX_LIST_ITEMS as u32,
                max_preview_cards: MAX_PREVIEW_CARDS,
                max_sleep_batches: MAX_SLEEP_BATCHES,
                max_sleep_proposals_per_batch: MAX_SLEEP_PROPOSALS_PER_BATCH,
            },
        },
    }
}

pub fn list_hazel_cards(store_limit: Option<u32>) -> ListHazelCardsResponse {
    let limit = bounded_limit(store_limit);
    ListHazelCardsResponse {
        cards: inspect_hazel_cards().into_iter().take(limit).collect(),
    }
}

pub fn get_hazel_card(card_id: &str) -> Option<HazelCardView> {
    inspect_hazel_cards()
        .into_iter()
        .find(|card| card.card_id == card_id)
}

pub fn list_hazel_proposal_batches(
    store: &Store,
    limit: Option<u32>,
) -> ListHazelProposalBatchesResponse {
    let limit = bounded_limit(limit);
    ListHazelProposalBatchesResponse {
        batches: store
            .list_hazel_proposal_batches()
            .into_iter()
            .map(map_batch_view)
            .take(limit)
            .collect(),
    }
}

pub fn get_hazel_proposal_batch(store: &Store, batch_id: &str) -> Option<HazelProposalBatchView> {
    store.get_hazel_proposal_batch(batch_id).map(map_batch_view)
}

pub fn list_hazel_sleep_jobs(store: &Store, limit: Option<u32>) -> ListHazelSleepJobsResponse {
    let limit = bounded_limit(limit);
    ListHazelSleepJobsResponse {
        jobs: store
            .list_hazel_sleep_jobs()
            .into_iter()
            .map(|(job_id, state, run_id, proposal_batch_ids, summary)| HazelSleepJobView {
                job_id,
                state,
                run_id,
                proposal_batch_ids,
                summary,
            })
            .take(limit)
            .collect(),
    }
}

pub fn get_hazel_sleep_job(store: &Store, job_id: &str) -> Option<HazelSleepJobView> {
    store
        .get_hazel_sleep_job(job_id)
        .map(|(job_id, state, run_id, proposal_batch_ids, summary)| HazelSleepJobView {
            job_id,
            state,
            run_id,
            proposal_batch_ids,
            summary,
        })
}

pub fn preview_hazel_retrieval(
    store: &mut Store,
    request: HazelRetrievalPreviewRequest,
) -> Result<HazelRetrievalPreviewResponse, String> {
    if request.input.top_k.unwrap_or(1) > MAX_PREVIEW_CARDS as usize {
        return Err(format!(
            "hazel_preview_invalid reason=top_k_exceeded actual={} max={}",
            request.input.top_k.unwrap_or(1),
            MAX_PREVIEW_CARDS
        ));
    }
    if request.input.token_budget.unwrap_or(1) > MAX_PREVIEW_TOKEN_BUDGET {
        return Err(format!(
            "hazel_preview_invalid reason=token_budget_exceeded actual={} max={}",
            request.input.token_budget.unwrap_or(1),
            MAX_PREVIEW_TOKEN_BUDGET
        ));
    }
    validate_policy_ids(&request.input.policy_ids)?;
    let payload = HazelMemoryCore::default().recollect(&request.input);
    let preview_id = store.record_hazel_preview(payload.clone())?;
    Ok(HazelRetrievalPreviewResponse { preview_id, payload })
}

pub fn validate_hazel_proposal_batch_action(
    store: &mut Store,
    request: ValidateHazelProposalBatchRequest,
) -> Result<ValidateHazelProposalBatchResponse, String> {
    validate_policy_ids(&request.strict_policy_ids)?;
    let batch = store
        .get_hazel_proposal_batch(&request.batch_id)
        .ok_or_else(|| format!("hazel_proposal_batch_not_found batch_id={}", request.batch_id))?;
    validate_proposal_batch(&batch)?;
    let summary = "proposal batch validated".to_string();
    let validation_id = store.record_hazel_validation(&request.batch_id, true, &summary)?;
    Ok(ValidateHazelProposalBatchResponse {
        validation_id,
        batch_id: request.batch_id,
        accepted: true,
        summary,
    })
}

pub fn submit_hazel_proposal_batch_action(
    store: &mut Store,
    request: SubmitHazelProposalBatchRequest,
) -> Result<SubmitHazelProposalBatchResponse, String> {
    validate_policy_ids(&request.strict_policy_ids)?;
    let batch = store
        .get_hazel_proposal_batch(&request.batch_id)
        .ok_or_else(|| format!("hazel_proposal_batch_not_found batch_id={}", request.batch_id))?;
    let _ = submit_proposal_batch(batch)?;
    let state = "accepted".to_string();
    let summary = "proposal batch accepted".to_string();
    let submission_id = store.record_hazel_submission(&request.batch_id, &state, &summary)?;
    Ok(SubmitHazelProposalBatchResponse {
        submission_id,
        batch_id: request.batch_id,
        state,
        summary,
    })
}

pub fn enqueue_hazel_sleep_job_action(
    store: &mut Store,
    request: EnqueueHazelSleepJobRequest,
) -> Result<EnqueueHazelSleepJobResponse, String> {
    let budget = SleepBudget {
        max_batches: request.max_batches as usize,
        max_proposals_per_batch: request.max_proposals_per_batch as usize,
    };
    validate_sleep_budget(&budget)?;
    let messages = request
        .messages
        .iter()
        .map(map_openai_message)
        .collect::<Vec<_>>();
    let batch = import_openai_messages_to_proposal_batch(
        &request.source_ref,
        &request.idempotency_key,
        &messages,
    )?;
    let run_id = derive_sleep_run_id(std::slice::from_ref(&batch))?;
    let output = SleepJobOutput {
        run_id: run_id.clone(),
        batches: vec![batch.clone()],
    };
    validate_sleep_output(&output, &budget)?;
    let batch_id = batch.batch_id.clone();
    store.record_hazel_proposal_batch(batch)?;
    let job_id = request
        .job_id
        .unwrap_or_else(|| format!("hazel-job-{}", request.idempotency_key));
    store.record_hazel_sleep_job(
        &job_id,
        HazelSleepJobState::Completed,
        Some(run_id),
        vec![batch_id.clone()],
        "completed with proposal batches",
    )?;
    Ok(EnqueueHazelSleepJobResponse {
        job: get_hazel_sleep_job(store, &job_id)
            .ok_or_else(|| format!("hazel_sleep_job_not_found job_id={job_id}"))?,
        proposal_batch_ids: vec![batch_id],
    })
}

pub fn cancel_hazel_sleep_job_action(
    store: &mut Store,
    job_id: &str,
) -> Result<CancelHazelSleepJobResponse, String> {
    let (job_id, state, run_id, proposal_batch_ids, summary) =
        store.update_hazel_sleep_job_state(job_id, HazelSleepJobState::Canceled, "sleep job canceled")?;
    Ok(CancelHazelSleepJobResponse {
        job: HazelSleepJobView {
            job_id,
            state,
            run_id,
            proposal_batch_ids,
            summary,
        },
    })
}

fn inspect_hazel_cards() -> Vec<HazelCardView> {
    HazelMemoryCore::default()
        .inspect_cards()
        .into_iter()
        .map(map_card_view)
        .collect()
}

fn map_card_view(card: RecollectionCard) -> HazelCardView {
    HazelCardView {
        card_id: card.card_id,
        kind: card.kind,
        state: card.state,
        subject: card.subject,
        text: card.text,
        provenance: card.provenance,
        policy_ids: card.policy_ids,
    }
}

fn map_batch_view(batch: ProposalBatch) -> HazelProposalBatchView {
    HazelProposalBatchView {
        batch_id: batch.batch_id,
        idempotency_key: batch.idempotency_key,
        source_ref: batch.provenance.source_ref,
        producer: batch.provenance.producer,
        proposal_count: batch.proposals.len().min(u32::MAX as usize) as u32,
    }
}

fn bounded_limit(limit: Option<u32>) -> usize {
    limit
        .map(|limit| limit as usize)
        .unwrap_or(MAX_LIST_ITEMS)
        .clamp(1, MAX_LIST_ITEMS)
}

fn validate_policy_ids(policy_ids: &[String]) -> Result<(), String> {
    let known = KNOWN_POLICY_IDS.iter().copied().collect::<BTreeSet<_>>();
    if let Some(policy_id) = policy_ids
        .iter()
        .find(|policy_id| !known.contains(policy_id.as_str()))
    {
        return Err(format!(
            "hazel_policy_unknown policy_id={} strict=true",
            policy_id
        ));
    }
    Ok(())
}

fn map_openai_message(message: &HazelConversationMessage) -> OpenAiMessage {
    OpenAiMessage {
        role: message.role.clone(),
        content: message.content.clone(),
    }
}

#[cfg(test)]
mod tests {
    use sharo_hazel_core::proposal::{BatchProvenance, Proposal, ProposalBatch, ProposalKind};
    use sharo_hazel_core::sleep::SleepJobOutput;
    use sharo_core::protocol::HazelSleepJobState;

    use super::*;

    fn temp_store(prefix: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{nanos}.json"))
    }

    fn seeded_store() -> Store {
        let path = temp_store("hazel-control-plane");
        let mut store = Store::open(&path).expect("open store");
        store
            .record_hazel_proposal_batch(ProposalBatch {
                batch_id: "batch-000001".to_string(),
                idempotency_key: "idemp-000001".to_string(),
                provenance: BatchProvenance {
                    source_ref: "note:hazel".to_string(),
                    producer: "operator".to_string(),
                },
                proposals: vec![Proposal {
                    proposal_id: "proposal-000001".to_string(),
                    kind: ProposalKind::ChunkUpsert,
                    chunk: Some(sharo_hazel_core::domain::Chunk {
                        chunk_id: "chunk-000001".to_string(),
                        content: "hazel inspection batch".to_string(),
                        source_ref: "note:hazel".to_string(),
                    }),
                    entity: None,
                    relation: None,
                    assertion: None,
                }],
            })
            .expect("record batch");
        store
            .record_hazel_sleep_job(
                "job-000001",
                HazelSleepJobState::Completed,
                Some(
                    SleepJobOutput {
                        run_id: "sleep-run-v2-seeded".to_string(),
                        batches: Vec::new(),
                    }
                    .run_id,
                ),
                vec!["batch-000001".to_string()],
                "completed with one batch",
            )
            .expect("record job");
        store
    }

    #[test]
    fn hazel_status_response_is_bounded() {
        let store = seeded_store();

        let response = get_hazel_status(&store);

        assert!(response.status.available);
        assert!(response.status.card_count <= response.status.limits.max_list_items);
        assert_eq!(response.status.proposal_batch_count, 1);
        assert_eq!(response.status.sleep_job_count, 1);
    }

    #[test]
    fn hazel_card_view_preserves_provenance_fields() {
        let response = list_hazel_cards(Some(1));

        assert_eq!(response.cards.len(), 1);
        assert!(!response.cards[0].provenance.is_empty());
        assert!(
            response.cards[0].provenance[0]
                .source_ref
                .starts_with("hazel:assertion/")
        );
    }

    #[test]
    fn hazel_inspection_responses_never_expose_unbounded_payloads() {
        let response = list_hazel_cards(Some(5_000));

        assert!(response.cards.len() <= MAX_LIST_ITEMS);
    }
}
