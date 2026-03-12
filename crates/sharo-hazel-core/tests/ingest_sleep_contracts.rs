use sharo_hazel_core::ingest::{
    ConversationFormat, ConversationImportOptions, ConversationPayload, OpenAiMessage,
    import_conversation_log, import_openai_messages_to_proposal_batch, submit_proposal_batch,
    submit_proposal_batches, validate_proposal_batch,
};
use sharo_hazel_core::proposal::{BatchProvenance, Proposal, ProposalBatch, ProposalKind};
use sharo_hazel_core::sleep::{
    SleepBudget, SleepJobOutput, derive_sleep_run_id, validate_sleep_budget, validate_sleep_output,
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
fn conversation_import_contract_maps_format_payload_to_batch_set() {
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
    let batches = import_conversation_log(
        ConversationFormat::OpenAiMessagesV1,
        "conversation:session-2",
        ConversationPayload::OpenAiMessagesV1(messages),
        ConversationImportOptions {
            idempotency_key: "idem-2".to_string(),
        },
    )
    .expect("import contract must succeed");
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].idempotency_key, "idem-2");
}

#[test]
fn conversation_import_canonicalizes_idempotency_key() {
    let batch = import_openai_messages_to_proposal_batch(
        "conversation:session-3",
        "  idem-4  ",
        &[OpenAiMessage {
            role: "user".to_string(),
            content: "remember hazel".to_string(),
        }],
    )
    .expect("import must succeed");
    assert_eq!(batch.batch_id, "import-idem-4");
    assert_eq!(batch.idempotency_key, "idem-4");
}

#[test]
fn conversation_import_rejects_unknown_message_roles() {
    let error = import_openai_messages_to_proposal_batch(
        "conversation:session-4",
        "idem-unknown-role",
        &[OpenAiMessage {
            role: "operator".to_string(),
            content: "remember hazel".to_string(),
        }],
    )
    .expect_err("unknown role must fail");
    assert!(error.contains("conversation_import_invalid_role"));
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
    let error = validate_proposal_batch(&batch).expect_err("must fail");
    assert!(error.contains("idempotency_key_required"));
    assert!(error.contains("proposal_submit_invalid"));
}

#[test]
fn proposal_submit_contracts_accept_valid_batches() {
    let batch = import_openai_messages_to_proposal_batch(
        "conversation:session-1",
        "idem-3",
        &[OpenAiMessage {
            role: "user".to_string(),
            content: "remember hazel".to_string(),
        }],
    )
    .expect("import must succeed");
    let accepted = submit_proposal_batch(batch.clone()).expect("single submit should pass");
    assert_eq!(accepted.batch_id, batch.batch_id);
    let accepted_many = submit_proposal_batches(vec![batch]).expect("multi submit should pass");
    assert_eq!(accepted_many.len(), 1);
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
    let batches = vec![batch];
    let output = SleepJobOutput {
        run_id: derive_sleep_run_id(&batches).expect("run id derivation should succeed"),
        batches,
    };
    let budget = SleepBudget {
        max_batches: 4,
        max_proposals_per_batch: 8,
    };
    validate_sleep_output(&output, &budget).expect("sleep output should validate");
}

#[test]
fn sleep_job_run_id_is_deterministic_for_same_batches() {
    let batch = import_openai_messages_to_proposal_batch(
        "conversation:session-1",
        "idem-5",
        &[OpenAiMessage {
            role: "user".to_string(),
            content: "remember hazel deterministically".to_string(),
        }],
    )
    .expect("import must succeed");
    let batches = vec![batch];
    let run_a = derive_sleep_run_id(&batches).expect("run id derivation should succeed");
    let run_b = derive_sleep_run_id(&batches).expect("run id derivation should succeed");
    assert_eq!(run_a, run_b);
}

#[test]
fn sleep_job_run_id_changes_when_proposal_content_changes() {
    let a = import_openai_messages_to_proposal_batch(
        "conversation:session-1",
        "idem-7",
        &[OpenAiMessage {
            role: "user".to_string(),
            content: "remember hazel".to_string(),
        }],
    )
    .expect("import must succeed");
    let b = import_openai_messages_to_proposal_batch(
        "conversation:session-1",
        "idem-7",
        &[OpenAiMessage {
            role: "user".to_string(),
            content: "remember hazel differently".to_string(),
        }],
    )
    .expect("import must succeed");
    let run_a = derive_sleep_run_id(&[a]).expect("run id derivation should succeed");
    let run_b = derive_sleep_run_id(&[b]).expect("run id derivation should succeed");
    assert_ne!(run_a, run_b);
}

#[test]
fn sleep_job_output_rejects_run_id_that_is_not_content_derived() {
    let batch = import_openai_messages_to_proposal_batch(
        "conversation:session-1",
        "idem-6",
        &[OpenAiMessage {
            role: "user".to_string(),
            content: "remember hazel".to_string(),
        }],
    )
    .expect("import must succeed");
    let output = SleepJobOutput {
        run_id: "sleep-run-random".to_string(),
        batches: vec![batch],
    };
    let budget = SleepBudget {
        max_batches: 4,
        max_proposals_per_batch: 8,
    };
    let error = validate_sleep_output(&output, &budget).expect_err("must fail");
    assert!(error.contains("run_id_mismatch"));
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

#[test]
fn bulk_submit_rejects_missing_batch_id() {
    let batch = ProposalBatch {
        batch_id: "  ".to_string(),
        idempotency_key: "idem-8".to_string(),
        provenance: BatchProvenance {
            source_ref: "conversation:session-8".to_string(),
            producer: "conversation-import/openai-messages-v1".to_string(),
        },
        proposals: vec![],
    };
    let error = validate_proposal_batch(&batch).expect_err("must fail");
    assert!(error.contains("batch_id_required"));
}

#[test]
fn bulk_submit_rejects_proposal_shape_mismatch() {
    let batch = ProposalBatch {
        batch_id: "batch-9".to_string(),
        idempotency_key: "idem-9".to_string(),
        provenance: BatchProvenance {
            source_ref: "conversation:session-9".to_string(),
            producer: "conversation-import/openai-messages-v1".to_string(),
        },
        proposals: vec![Proposal {
            proposal_id: "proposal-1".to_string(),
            kind: ProposalKind::ChunkUpsert,
            chunk: None,
            entity: None,
            relation: None,
            assertion: None,
        }],
    };
    let error = validate_proposal_batch(&batch).expect_err("must fail");
    assert!(error.contains("proposal_shape_mismatch"));
}
