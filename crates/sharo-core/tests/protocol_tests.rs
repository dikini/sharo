use proptest::prelude::*;
use sharo_core::protocol::{
    EffectivePolicyBundle, HookSchemaDescriptor, ObjectSchema, PolicyMergeMode, PolicyRule,
    PrePromptComposeHookInput, ProvenanceRef, RecollectionCard, RecollectionCardKind,
    RecollectionCardState, RecollectionLintLimits, RecollectionPayload, SubmitTaskRequest,
    SubmitTaskResponse, TaskState, TaskStatusRequest, TaskStatusResponse, TaskSummary,
    expected_pre_prompt_compose_input_schema, expected_recollection_output_schema,
    input_schema_compatible, object_schema_well_formed, output_schema_compatible,
    semantic_lint_recollection_payload_with_limits, validate_pre_prompt_compose_input_value,
    validate_recollection_payload_value,
};

#[test]
fn submit_request_response_roundtrip() {
    let request = SubmitTaskRequest {
        session_id: Some("session-a".to_string()),
        goal: "read docs".to_string(),
    };

    let response = SubmitTaskResponse {
        task_id: "task-0001".to_string(),
        state: TaskState::Submitted,
    };

    assert_eq!(request.session_id.as_deref(), Some("session-a"));
    assert_eq!(request.goal, "read docs");
    assert_eq!(response.task_id, "task-0001");
    assert_eq!(response.state, TaskState::Submitted);
}

#[test]
fn status_request_response_roundtrip() {
    let request = TaskStatusRequest {
        task_id: "task-0007".to_string(),
    };

    let response = TaskStatusResponse {
        task_id: request.task_id.clone(),
        state: TaskState::Running,
        summary: "in progress".to_string(),
    };

    assert_eq!(response.task_id, "task-0007");
    assert_eq!(response.state, TaskState::Running);
    assert_eq!(response.summary, "in progress");
}

#[test]
fn protocol_includes_optional_coordination_summary() {
    let task = TaskSummary {
        task_id: "task-42".to_string(),
        session_id: "session-1".to_string(),
        task_state: "awaiting_approval".to_string(),
        current_step_summary: "restricted write pending approval".to_string(),
        blocking_reason: Some("approval_required approval_id=approval-000001".to_string()),
        coordination_summary: Some(
            "conflict_id=conflict-000001 scope=notes related_task_id=task-41".to_string(),
        ),
        result_preview: None,
    };

    let payload = serde_json::to_string(&task).expect("serialize task summary");
    let roundtrip: TaskSummary = serde_json::from_str(&payload).expect("parse task summary");
    assert_eq!(roundtrip.coordination_summary, task.coordination_summary);
}

#[test]
fn recollection_payload_roundtrip_preserves_policy_ids_and_cards() {
    let payload = RecollectionPayload {
        policy_ids: vec!["hunch.v1".to_string(), "safety.strict.v1".to_string()],
        cards: vec![RecollectionCard {
            card_id: "card-1".to_string(),
            kind: RecollectionCardKind::AssociationCue,
            state: RecollectionCardState::Candidate,
            subject: "hazel name origin".to_string(),
            text: "Hazel may signal a wisdom-themed hunch.".to_string(),
            provenance: vec![ProvenanceRef {
                source_ref: "note:sharo/munin-memory-inspiration.md".to_string(),
                source_excerpt: Some("inspired by Muninn".to_string()),
            }],
            policy_ids: vec!["hunch.v1".to_string()],
        }],
    };

    let encoded = serde_json::to_string(&payload).expect("serialize recollection payload");
    let decoded: RecollectionPayload =
        serde_json::from_str(&encoded).expect("deserialize recollection payload");
    assert_eq!(decoded.policy_ids, payload.policy_ids);
    assert_eq!(decoded.cards, payload.cards);
}

#[test]
fn effective_policy_bundle_dedupes_and_sorts_policy_ids() {
    let bundle = EffectivePolicyBundle::new(
        vec![
            "safety.strict.v1".to_string(),
            "hunch.v1".to_string(),
            "hunch.v1".to_string(),
        ],
        PolicyMergeMode::StrictestWins,
        vec![PolicyRule::LabelGuesses, PolicyRule::PreferSupportedFacts],
    );

    assert_eq!(
        bundle.effective_policy_ids,
        vec!["hunch.v1".to_string(), "safety.strict.v1".to_string()]
    );
}

#[test]
fn pre_prompt_compose_hook_input_rejects_unknown_fields() {
    let raw = serde_json::json!({
        "session_id": "session-1",
        "task_id": "task-1",
        "goal": "answer memory question",
        "runtime": "daemon",
        "unexpected": true
    });

    let error = serde_json::from_value::<PrePromptComposeHookInput>(raw)
        .expect_err("unknown fields should be rejected");
    assert!(error.to_string().contains("unknown field"));
}

#[test]
fn pre_prompt_input_validation_accepts_optional_policy_sections() {
    let raw = serde_json::json!({
        "session_id": "session-1",
        "task_id": "task-1",
        "goal": "answer memory question",
        "runtime": "daemon",
        "policy_ids": ["hunch.v1"],
        "card_policy_hints": [
            {
                "kind": "association_cue",
                "policy_ids": ["hunch.v1"],
                "max_cards": 3
            }
        ]
    });
    let parsed = validate_pre_prompt_compose_input_value(&raw).expect("valid input");
    assert_eq!(parsed.policy_ids, vec!["hunch.v1".to_string()]);
    assert_eq!(parsed.card_policy_hints.len(), 1);
}

#[test]
fn recollection_payload_validation_rejects_missing_provenance() {
    let raw = serde_json::json!({
        "policy_ids": ["hunch.v1"],
        "cards": [{
            "card_id": "card-1",
            "kind": "soft_recollection",
            "state": "candidate",
            "subject": "hazel",
            "text": "x",
            "provenance": [],
            "policy_ids": ["hunch.v1"]
        }]
    });
    let error = validate_recollection_payload_value(&raw).expect_err("must fail");
    assert!(error.contains("missing_provenance"));
}

#[test]
fn shared_hook_schema_compatibility_checks_work() {
    let expected_input = expected_pre_prompt_compose_input_schema();
    let tool_input = ObjectSchema::new(
        &["session_id", "task_id", "goal"],
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
    );
    assert!(input_schema_compatible(&expected_input, &tool_input));

    let expected_output = expected_recollection_output_schema();
    let tool_output = ObjectSchema::new(
        &["policy_ids", "cards"],
        &["policy_ids", "cards", "extra"],
        false,
    );
    assert!(!output_schema_compatible(&expected_output, &tool_output));
}

#[test]
fn input_schema_compatibility_rejects_tool_allow_additional_when_expected_is_strict() {
    let expected_input = expected_pre_prompt_compose_input_schema();
    let permissive_tool_input = ObjectSchema::new(
        &["session_id", "task_id", "goal"],
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
        true,
    );
    assert!(!input_schema_compatible(
        &expected_input,
        &permissive_tool_input
    ));
}

#[test]
fn output_schema_compatibility_rejects_tool_allow_additional_when_expected_is_strict() {
    let expected_output = expected_recollection_output_schema();
    let permissive_tool_output =
        ObjectSchema::new(&["policy_ids", "cards"], &["policy_ids", "cards"], true);
    assert!(!output_schema_compatible(
        &expected_output,
        &permissive_tool_output
    ));
}

#[test]
fn schema_compatibility_rejects_malformed_tool_schema_definition() {
    let malformed = ObjectSchema::new(&["cards"], &["policy_ids"], false);
    assert!(!object_schema_well_formed(&malformed));
    assert!(!input_schema_compatible(
        &expected_pre_prompt_compose_input_schema(),
        &malformed
    ));
    assert!(!output_schema_compatible(
        &expected_recollection_output_schema(),
        &malformed
    ));
}

#[test]
fn recollection_payload_semantic_lint_enforces_limits() {
    let payload = RecollectionPayload {
        policy_ids: vec!["hunch.v1".to_string()],
        cards: vec![RecollectionCard {
            card_id: "card-1".to_string(),
            kind: RecollectionCardKind::AssociationCue,
            state: RecollectionCardState::Candidate,
            subject: "hazel".to_string(),
            text: "word ".repeat(500),
            provenance: vec![ProvenanceRef {
                source_ref: "note:hazel".to_string(),
                source_excerpt: None,
            }],
            policy_ids: vec!["hunch.v1".to_string()],
        }],
    };
    let limits = RecollectionLintLimits {
        max_cards: 2,
        max_payload_bytes: 10_000,
        max_tokens: 32,
    };
    let error =
        semantic_lint_recollection_payload_with_limits(&payload, &limits).expect_err("must fail");
    assert!(error.contains("max_tokens_exceeded"));
}

#[test]
fn hook_schema_descriptor_roundtrip() {
    let descriptor = HookSchemaDescriptor {
        input: expected_pre_prompt_compose_input_schema(),
        output: expected_recollection_output_schema(),
    };
    let encoded = serde_json::to_string(&descriptor).expect("serialize");
    let decoded: HookSchemaDescriptor = serde_json::from_str(&encoded).expect("deserialize");
    assert_eq!(decoded, descriptor);
}

fn object_schema_strategy() -> impl Strategy<Value = ObjectSchema> {
    let key = "[a-z_]{1,16}";
    (
        prop::collection::btree_set(key, 0..8),
        prop::collection::btree_set(key, 0..8),
        any::<bool>(),
    )
        .prop_map(|(required, allowed, allow_additional)| ObjectSchema {
            required,
            allowed,
            allow_additional,
        })
}

proptest! {
    #[test]
    fn prop_input_schema_compatibility_is_deterministic_for_same_schemas(
        expected in object_schema_strategy(),
        tool in object_schema_strategy(),
    ) {
        let a = input_schema_compatible(&expected, &tool);
        let b = input_schema_compatible(&expected, &tool);
        prop_assert_eq!(a, b);
    }

    #[test]
    fn prop_output_schema_compatibility_is_deterministic_for_same_schemas(
        expected in object_schema_strategy(),
        tool in object_schema_strategy(),
    ) {
        let a = output_schema_compatible(&expected, &tool);
        let b = output_schema_compatible(&expected, &tool);
        prop_assert_eq!(a, b);
    }

    #[test]
    fn prop_object_schema_well_formed_rejects_required_not_in_allowed_when_strict(
        required in prop::collection::btree_set("[a-z_]{1,16}", 1..8),
        allowed in prop::collection::btree_set("[a-z_]{1,16}", 0..8),
    ) {
        prop_assume!(!required.is_subset(&allowed));
        let schema = ObjectSchema {
            required,
            allowed,
            allow_additional: false,
        };
        prop_assert!(!object_schema_well_formed(&schema));
    }

    #[test]
    fn prop_protocol_roundtrip_preserves_task_summary_fields(
        task_id in "[a-z0-9\\-]{1,24}",
        session_id in "[a-z0-9\\-]{1,24}",
        task_state in "[a-z_]{1,24}",
        summary in ".{0,64}",
    ) {
        let task = TaskSummary {
            task_id: task_id.clone(),
            session_id: session_id.clone(),
            task_state: task_state.clone(),
            current_step_summary: summary.clone(),
            blocking_reason: Some("reason".to_string()),
            coordination_summary: Some("coord".to_string()),
            result_preview: Some("preview".to_string()),
        };

        let payload = serde_json::to_string(&task).expect("serialize task summary");
        let roundtrip: TaskSummary = serde_json::from_str(&payload).expect("parse task summary");

        prop_assert_eq!(roundtrip.task_id, task_id);
        prop_assert_eq!(roundtrip.session_id, session_id);
        prop_assert_eq!(roundtrip.task_state, task_state);
        prop_assert_eq!(roundtrip.current_step_summary, summary);
        prop_assert_eq!(roundtrip.result_preview, Some("preview".to_string()));
    }
}
