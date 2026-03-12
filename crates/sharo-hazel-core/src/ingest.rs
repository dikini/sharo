use crate::domain::Chunk;
use crate::proposal::{BatchProvenance, Proposal, ProposalBatch, ProposalKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversationFormat {
    OpenAiMessagesV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAiMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationImportOptions {
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversationPayload {
    OpenAiMessagesV1(Vec<OpenAiMessage>),
}

pub fn import_openai_messages_to_proposal_batch(
    source_ref: &str,
    idempotency_key: &str,
    messages: &[OpenAiMessage],
) -> Result<ProposalBatch, String> {
    if source_ref.trim().is_empty() {
        return Err("conversation_import_source_ref_required".to_string());
    }
    let canonical_idempotency_key = idempotency_key.trim();
    if canonical_idempotency_key.is_empty() {
        return Err("conversation_import_idempotency_key_required".to_string());
    }

    let canonical_messages = messages
        .iter()
        .map(canonicalize_openai_message)
        .collect::<Result<Vec<_>, _>>()?;

    let proposals: Vec<Proposal> = canonical_messages
        .iter()
        .enumerate()
        .filter_map(|(index, message)| {
            let content = message.content.trim();
            if content.is_empty() {
                return None;
            }
            Some(Proposal {
                proposal_id: format!("msg-{index}"),
                kind: ProposalKind::ChunkUpsert,
                chunk: Some(Chunk {
                    chunk_id: format!("chunk-{index}"),
                    content: format!("{}: {}", message.role, content),
                    source_ref: source_ref.to_string(),
                }),
                entity: None,
                relation: None,
                assertion: None,
            })
        })
        .collect();

    Ok(ProposalBatch {
        batch_id: format!("import-{canonical_idempotency_key}"),
        idempotency_key: canonical_idempotency_key.to_string(),
        provenance: BatchProvenance {
            source_ref: source_ref.to_string(),
            producer: "conversation-import/openai-messages-v1".to_string(),
        },
        proposals,
    })
}

fn canonicalize_openai_message(message: &OpenAiMessage) -> Result<OpenAiMessage, String> {
    let role = message.role.trim().to_ascii_lowercase();
    if !matches!(role.as_str(), "system" | "user" | "assistant" | "tool") {
        return Err(format!(
            "conversation_import_invalid_role role={}",
            message.role.trim()
        ));
    }
    Ok(OpenAiMessage {
        role,
        content: message.content.clone(),
    })
}

pub fn import_conversation_log(
    format: ConversationFormat,
    source_ref: &str,
    payload: ConversationPayload,
    options: ConversationImportOptions,
) -> Result<Vec<ProposalBatch>, String> {
    if options.idempotency_key.trim().is_empty() {
        return Err("conversation_import_idempotency_key_required".to_string());
    }
    match (format, payload) {
        (ConversationFormat::OpenAiMessagesV1, ConversationPayload::OpenAiMessagesV1(messages)) => {
            let batch = import_openai_messages_to_proposal_batch(
                source_ref,
                options.idempotency_key.trim(),
                &messages,
            )?;
            Ok(vec![batch])
        }
    }
}

pub fn validate_proposal_batch(batch: &ProposalBatch) -> Result<(), String> {
    if batch.batch_id.trim().is_empty() {
        return Err("proposal_submit_invalid reason=batch_id_required".to_string());
    }
    if batch.idempotency_key.trim().is_empty() {
        return Err(format!(
            "proposal_submit_invalid reason=idempotency_key_required batch_id={}",
            batch.batch_id
        ));
    }
    if batch.provenance.source_ref.trim().is_empty() || batch.provenance.producer.trim().is_empty()
    {
        return Err(format!(
            "proposal_submit_invalid reason=provenance_required batch_id={}",
            batch.batch_id
        ));
    }
    for proposal in &batch.proposals {
        validate_proposal_shape(proposal).map_err(|reason| {
            format!(
                "proposal_submit_invalid reason={} batch_id={} proposal_id={}",
                reason, batch.batch_id, proposal.proposal_id
            )
        })?;
    }
    Ok(())
}

pub fn submit_proposal_batch(batch: ProposalBatch) -> Result<ProposalBatch, String> {
    validate_proposal_batch(&batch)?;
    Ok(batch)
}

pub fn submit_proposal_batches(batches: Vec<ProposalBatch>) -> Result<Vec<ProposalBatch>, String> {
    validate_proposal_batches(&batches)?;
    Ok(batches)
}

pub fn validate_proposal_batches(batches: &[ProposalBatch]) -> Result<(), String> {
    for batch in batches {
        validate_proposal_batch(batch)?;
    }
    Ok(())
}

fn validate_proposal_shape(proposal: &Proposal) -> Result<(), &'static str> {
    if proposal.proposal_id.trim().is_empty() {
        return Err("proposal_id_required");
    }
    match proposal.kind {
        ProposalKind::ChunkUpsert => {
            if proposal.chunk.is_none()
                || proposal.entity.is_some()
                || proposal.relation.is_some()
                || proposal.assertion.is_some()
            {
                return Err("proposal_shape_mismatch");
            }
        }
        ProposalKind::EntityUpsert => {
            if proposal.entity.is_none()
                || proposal.chunk.is_some()
                || proposal.relation.is_some()
                || proposal.assertion.is_some()
            {
                return Err("proposal_shape_mismatch");
            }
        }
        ProposalKind::RelationUpsert => {
            if proposal.relation.is_none()
                || proposal.chunk.is_some()
                || proposal.entity.is_some()
                || proposal.assertion.is_some()
            {
                return Err("proposal_shape_mismatch");
            }
        }
        ProposalKind::AssertionUpsert => {
            if proposal.assertion.is_none()
                || proposal.chunk.is_some()
                || proposal.entity.is_some()
                || proposal.relation.is_some()
            {
                return Err("proposal_shape_mismatch");
            }
        }
    }
    Ok(())
}
