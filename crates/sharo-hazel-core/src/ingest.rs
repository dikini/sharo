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

pub fn import_openai_messages_to_proposal_batch(
    source_ref: &str,
    idempotency_key: &str,
    messages: &[OpenAiMessage],
) -> Result<ProposalBatch, String> {
    if source_ref.trim().is_empty() {
        return Err("conversation_import_source_ref_required".to_string());
    }
    if idempotency_key.trim().is_empty() {
        return Err("conversation_import_idempotency_key_required".to_string());
    }

    let proposals: Vec<Proposal> = messages
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
        batch_id: format!("import-{}", idempotency_key.trim()),
        idempotency_key: idempotency_key.to_string(),
        provenance: BatchProvenance {
            source_ref: source_ref.to_string(),
            producer: "conversation-import/openai-messages-v1".to_string(),
        },
        proposals,
    })
}

pub fn validate_bulk_submit_batch(batch: &ProposalBatch) -> Result<(), String> {
    if batch.idempotency_key.trim().is_empty() {
        return Err(format!(
            "bulk_submit_invalid reason=idempotency_key_required batch_id={}",
            batch.batch_id
        ));
    }
    if batch.provenance.source_ref.trim().is_empty() || batch.provenance.producer.trim().is_empty()
    {
        return Err(format!(
            "bulk_submit_invalid reason=provenance_required batch_id={}",
            batch.batch_id
        ));
    }
    Ok(())
}

pub fn validate_bulk_submit_batches(batches: &[ProposalBatch]) -> Result<(), String> {
    for batch in batches {
        validate_bulk_submit_batch(batch)?;
    }
    Ok(())
}
