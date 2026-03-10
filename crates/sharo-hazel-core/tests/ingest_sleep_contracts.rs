use sharo_hazel_core::ingest::{
    OpenAiMessage, import_openai_messages_to_proposal_batch, validate_bulk_submit_batch,
};
use sharo_hazel_core::proposal::{BatchProvenance, ProposalBatch};
use sharo_hazel_core::sleep::{
    SleepBudget, SleepJobOutput, validate_sleep_budget, validate_sleep_output,
};

#[test]
fn conversation_import_adapter_maps_openai_messages_to_proposal_batch() {
    let messages = vec![
        OpenAiMessage {
            role: "user".to_string(),
            content: "How does Hazel memory work?".to_string(),
        },
        OpenAiMessage {
            role: "assistant".to_string(),
            content: "It uses structured recollection cards.".to_string(),
        },
    ];
    let batch =
        import_openai_messages_to_proposal_batch("conversation:session-1", "idem-1", &messages)
            .expect("import must succeed");
    assert_eq!(batch.idempotency_key, "idem-1");
    assert_eq!(batch.provenance.source_ref, "conversation:session-1");
    assert_eq!(batch.proposals.len(), 2);
}

#[test]
fn bulk_submit_requires_idempotency_key_and_batch_provenance() {
    let batch = ProposalBatch {
        batch_id: "b-1".to_string(),
        idempotency_key: "".to_string(),
        provenance: BatchProvenance {
            source_ref: "".to_string(),
            producer: "".to_string(),
        },
        proposals: vec![],
    };
    let error = validate_bulk_submit_batch(&batch).expect_err("must fail");
    assert!(error.contains("idempotency_key_required"));
}

#[test]
fn sleep_job_output_must_be_proposal_batches_only() {
    let batch = import_openai_messages_to_proposal_batch(
        "conversation:session-1",
        "idem-1",
        &[OpenAiMessage {
            role: "user".to_string(),
            content: "remember hazel".to_string(),
        }],
    )
    .expect("import must succeed");
    let output = SleepJobOutput {
        run_id: "sleep-run-1".to_string(),
        batches: vec![batch],
    };
    let budget = SleepBudget {
        max_batches: 4,
        max_proposals_per_batch: 8,
    };
    validate_sleep_output(&output, &budget).expect("sleep output should validate");
}

#[test]
fn sleep_job_requires_bounded_budget_configuration() {
    let budget = SleepBudget {
        max_batches: 0,
        max_proposals_per_batch: 10,
    };
    let error = validate_sleep_budget(&budget).expect_err("must fail");
    assert!(error.contains("max_batches_zero"));
}
