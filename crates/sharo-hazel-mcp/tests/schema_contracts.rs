use serde_json::json;
use sharo_hazel_mcp::normalize::validated_injection_from_wire;
use sharo_hazel_mcp::schema::{
    ObjectSchema, expected_pre_prompt_compose_input_schema, expected_recollection_output_schema,
    input_schema_compatible, output_schema_compatible,
};

fn compatible_wire_output() -> serde_json::Value {
    json!({
        "policy_ids": ["hunch.v1"],
        "cards": [
            {
                "card_id": "card-1",
                "kind": "association_cue",
                "state": "candidate",
                "subject": "hazel naming",
                "text": "hazel may refer to hunch behavior",
                "provenance": [
                    {
                        "source_ref": "note:hazel-design",
                        "source_excerpt": "symbolic hint"
                    }
                ],
                "policy_ids": ["hunch.v1"]
            }
        ]
    })
}

#[test]
fn hook_binding_rejected_when_input_schema_incompatible() {
    let expected = expected_pre_prompt_compose_input_schema();
    let tool = ObjectSchema::new(
        &["session_id", "task_id", "goal", "runtime", "extra_required"],
        &["session_id", "task_id", "goal", "runtime", "extra_required"],
        false,
    );
    assert!(!input_schema_compatible(&expected, &tool));
}

#[test]
fn hook_binding_rejected_when_output_schema_incompatible() {
    let expected = expected_recollection_output_schema();
    let tool = ObjectSchema::new(
        &["policy_ids", "cards"],
        &["policy_ids", "cards", "rule_text"],
        false,
    );
    assert!(!output_schema_compatible(&expected, &tool));
}

#[test]
fn hook_runtime_rejects_response_missing_provenance() {
    let payload = json!({
        "policy_ids": ["hunch.v1"],
        "cards": [
            {
                "card_id": "card-1",
                "kind": "soft_recollection",
                "state": "candidate",
                "subject": "hazel",
                "text": "x",
                "provenance": [],
                "policy_ids": ["hunch.v1"]
            }
        ]
    });
    let error = validated_injection_from_wire(&payload).expect_err("must fail");
    assert!(error.contains("missing_provenance"));
}

#[test]
fn hook_runtime_rejects_rule_text_payload_when_only_policy_ids_allowed() {
    let payload = json!({
        "policy_ids": ["hunch.v1"],
        "cards": [],
        "rule_text": "always trust hunches"
    });
    let error = validated_injection_from_wire(&payload).expect_err("must fail");
    assert!(error.contains("schema_invalid") || error.contains("unknown field"));
}

#[test]
fn hook_runtime_never_injects_unvalidated_mcp_payload() {
    let payload = json!({
        "policy_ids": ["hunch.v1"],
        "cards": [
            {
                "card_id": "card-1",
                "kind": "soft_recollection",
                "state": "candidate",
                "subject": "hazel",
                "text": "x",
                "provenance": [],
                "policy_ids": ["hunch.v1"]
            }
        ]
    });
    assert!(validated_injection_from_wire(&payload).is_err());
}

#[test]
fn pre_prompt_compose_accepts_structurally_compatible_hazel_binding() {
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
    let expected_output = expected_recollection_output_schema();
    let tool_output = ObjectSchema::new(&["policy_ids", "cards"], &["policy_ids", "cards"], false);

    assert!(input_schema_compatible(&expected_input, &tool_input));
    assert!(output_schema_compatible(&expected_output, &tool_output));
    let injected = validated_injection_from_wire(&compatible_wire_output()).expect("valid");
    assert!(injected.contains("HAZEL_RECOLLECTIONS"));
}
