use sharo_core::protocol::{
    HazelCardPolicyHint, PrePromptComposeHookInput, RecollectionCardKind, RecollectionLintLimits,
    semantic_lint_recollection_payload_with_limits,
};
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

#[test]
fn hazel_retrieval_enforces_global_top_k_across_hints() {
    let core = HazelMemoryCore::default();
    let payload = core.recollect(&PrePromptComposeHookInput {
        session_id: "s1".to_string(),
        task_id: "t1".to_string(),
        goal: "hazel memory structured subsystem".to_string(),
        runtime: "daemon".to_string(),
        top_k: Some(1),
        token_budget: Some(256),
        relevance_threshold: Some(0.0),
        policy_ids: vec!["hunch.v1".to_string()],
        card_policy_hints: vec![
            HazelCardPolicyHint {
                kind: RecollectionCardKind::AssociationCue,
                policy_ids: vec!["hunch.v1".to_string()],
                max_cards: Some(2),
            },
            HazelCardPolicyHint {
                kind: RecollectionCardKind::SupportingContext,
                policy_ids: vec!["hunch.v1".to_string()],
                max_cards: Some(2),
            },
        ],
    });
    assert_eq!(payload.cards.len(), 1);
}

#[test]
fn hazel_retrieval_fallback_respects_token_budget() {
    let core = HazelMemoryCore::default();
    let payload = core.recollect(&PrePromptComposeHookInput {
        session_id: "s1".to_string(),
        task_id: "t1".to_string(),
        goal: "very long goal ".repeat(32),
        runtime: "daemon".to_string(),
        top_k: Some(2),
        token_budget: Some(16),
        relevance_threshold: Some(1.0),
        policy_ids: vec![],
        card_policy_hints: vec![],
    });
    let limits = RecollectionLintLimits {
        max_cards: 2,
        max_payload_bytes: 4_096,
        max_tokens: 16,
    };
    semantic_lint_recollection_payload_with_limits(&payload, &limits)
        .expect("fallback payload should stay within token budget");
}
