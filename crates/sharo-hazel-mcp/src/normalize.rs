use serde_json::Value;
use sharo_core::protocol::{
    RecollectionPayload, semantic_lint_recollection_payload, validate_recollection_payload_value,
};

pub fn normalize_recollection_output(payload: &Value) -> Result<RecollectionPayload, String> {
    validate_recollection_payload_value(payload)
}

pub fn semantic_lint_recollection(payload: &RecollectionPayload) -> Result<(), String> {
    semantic_lint_recollection_payload(payload)
}

pub fn validated_injection_from_wire(payload: &Value) -> Result<String, String> {
    let recollection = normalize_recollection_output(payload)?;
    let mut lines = Vec::new();
    lines.push("HAZEL_RECOLLECTIONS:".to_string());
    lines.push(format!("POLICY_IDS: {}", recollection.policy_ids.join(",")));
    lines.push(format!("CARD_COUNT: {}", recollection.cards.len()));
    Ok(lines.join("\n"))
}
