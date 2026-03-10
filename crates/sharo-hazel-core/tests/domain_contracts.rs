use sharo_hazel_core::domain::{
    Assertion, AssertionState, Association, association_implies_relation,
};
use sharo_hazel_core::lifecycle::resolve_state;

#[test]
fn hazel_core_rejects_unknown_fields_in_canonical_sections() {
    let raw = serde_json::json!({
        "entity_id": "entity-1",
        "label": "Hazel",
        "kind": "concept",
        "unknown": true
    });

    let error = serde_json::from_value::<sharo_hazel_core::domain::Entity>(raw)
        .expect_err("unknown fields must be rejected");
    assert!(error.to_string().contains("unknown field"));
}

#[test]
fn hazel_core_preserves_assertion_lineage_on_derived_assertions() {
    let base = Assertion {
        assertion_id: "a-1".to_string(),
        subject: "hazel".to_string(),
        predicate: "inspired_by".to_string(),
        object: "munin".to_string(),
        lineage: vec!["seed-1".to_string()],
        support_count: 4,
        contradiction_count: 0,
        confidence_milli: 880,
        state: AssertionState::Candidate,
    };

    let derived = Assertion::derive_from(&base, "a-2", "maps_to", "hunch");
    assert_eq!(
        derived.lineage,
        vec!["seed-1".to_string(), "a-1".to_string()]
    );
    assert_eq!(derived.state, AssertionState::Candidate);
}

#[test]
fn hazel_core_association_does_not_imply_relation() {
    let association = Association {
        association_id: "assoc-1".to_string(),
        from_entity_id: "entity-hazel".to_string(),
        to_entity_id: "entity-memory".to_string(),
        coactivation_count: 9,
    };

    assert!(!association_implies_relation(&association));
}

#[test]
fn hazel_core_candidate_to_active_transition_is_formula_driven() {
    let assertion = Assertion {
        assertion_id: "a-1".to_string(),
        subject: "hazel".to_string(),
        predicate: "supports".to_string(),
        object: "memory-design".to_string(),
        lineage: vec!["seed-1".to_string()],
        support_count: 2,
        contradiction_count: 0,
        confidence_milli: 900,
        state: AssertionState::Candidate,
    };

    let decision = resolve_state(&assertion);
    assert_eq!(decision.from, AssertionState::Candidate);
    assert_eq!(decision.to, AssertionState::Active);
    assert_eq!(decision.score_milli, 1_100);
}
