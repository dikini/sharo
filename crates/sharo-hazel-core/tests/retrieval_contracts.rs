use sharo_core::protocol::{HazelCardPolicyHint, PrePromptComposeHookInput, RecollectionCardKind};
use sharo_hazel_core::retrieval::HazelMemoryCore;

#[test]
fn hazel_retrieval_returns_cards_for_relevant_goal() {
    let core = HazelMemoryCore::default();
    let payload = core.recollect(&PrePromptComposeHookInput {
        session_id: "s1".to_string(),
        task_id: "t1".to_string(),
        goal: "memory subsystem architecture".to_string(),
        runtime: "daemon".to_string(),
        top_k: Some(2),
        token_budget: Some(256),
        relevance_threshold: Some(0.1),
        policy_ids: vec!["hunch.v1".to_string()],
        card_policy_hints: vec![HazelCardPolicyHint {
            kind: RecollectionCardKind::AssociationCue,
            policy_ids: vec!["hunch.v1".to_string()],
            max_cards: Some(1),
        }],
    });
    assert_eq!(payload.policy_ids, vec!["hunch.v1".to_string()]);
    assert_eq!(payload.cards.len(), 1);
    assert!(
        payload.cards[0]
            .text
            .contains("goal=memory subsystem architecture")
    );
}

#[test]
fn hazel_retrieval_emits_fallback_when_threshold_too_strict() {
    let core = HazelMemoryCore::default();
    let payload = core.recollect(&PrePromptComposeHookInput {
        session_id: "s1".to_string(),
        task_id: "t1".to_string(),
        goal: "completely unrelated vocabulary".to_string(),
        runtime: "daemon".to_string(),
        top_k: Some(2),
        token_budget: None,
        relevance_threshold: Some(1.0),
        policy_ids: vec!["hunch.v1".to_string()],
        card_policy_hints: vec![],
    });
    assert_eq!(payload.cards.len(), 1);
    assert!(payload.cards[0].card_id.contains("fallback"));
}
