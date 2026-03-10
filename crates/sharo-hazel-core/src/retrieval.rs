use sharo_core::protocol::{
    HazelCardPolicyHint, PrePromptComposeHookInput, ProvenanceRef, RecollectionCard,
    RecollectionCardKind, RecollectionCardState, RecollectionPayload, estimate_policy_id_tokens,
    estimate_recollection_card_tokens,
};

use crate::domain::{Assertion, AssertionState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HazelMemoryCore {
    assertions: Vec<Assertion>,
}

impl Default for HazelMemoryCore {
    fn default() -> Self {
        Self {
            assertions: vec![
                Assertion {
                    assertion_id: "hazel-memory-1".to_string(),
                    subject: "hazel".to_string(),
                    predicate: "is".to_string(),
                    object: "structured memory subsystem for sharo".to_string(),
                    lineage: vec!["seed".to_string()],
                    support_count: 5,
                    contradiction_count: 0,
                    confidence_milli: 920,
                    state: AssertionState::Active,
                },
                Assertion {
                    assertion_id: "hazel-memory-2".to_string(),
                    subject: "hazel".to_string(),
                    predicate: "supports".to_string(),
                    object: "policy-driven recollection cards with provenance".to_string(),
                    lineage: vec!["seed".to_string()],
                    support_count: 4,
                    contradiction_count: 0,
                    confidence_milli: 870,
                    state: AssertionState::Candidate,
                },
            ],
        }
    }
}

impl HazelMemoryCore {
    pub fn recollect(&self, input: &PrePromptComposeHookInput) -> RecollectionPayload {
        let top_k = input.top_k.unwrap_or(3).max(1);
        let token_budget = input.token_budget.unwrap_or(usize::MAX).max(1);
        let mut cards = Vec::new();
        let mut used_tokens = estimate_policy_id_tokens(&input.policy_ids);
        let hints = if input.card_policy_hints.is_empty() {
            vec![HazelCardPolicyHint {
                kind: RecollectionCardKind::AssociationCue,
                policy_ids: input.policy_ids.clone(),
                max_cards: Some(top_k),
            }]
        } else {
            input.card_policy_hints.clone()
        };

        for hint in hints {
            if cards.len() >= top_k {
                break;
            }
            let max_for_hint = hint
                .max_cards
                .unwrap_or(top_k)
                .max(1)
                .min(top_k.saturating_sub(cards.len()));
            let mut added = 0usize;
            for assertion in &self.assertions {
                if added >= max_for_hint || cards.len() >= top_k {
                    break;
                }
                if !is_relevant(
                    assertion,
                    &input.goal,
                    input.relevance_threshold.unwrap_or(0.0),
                ) {
                    continue;
                }
                let card = assertion_to_card(assertion, &hint, &input.goal);
                let card_tokens = estimate_recollection_card_tokens(&card);
                if used_tokens.saturating_add(card_tokens) > token_budget {
                    continue;
                }
                used_tokens = used_tokens.saturating_add(card_tokens);
                cards.push(card);
                added += 1;
            }
        }

        if cards.is_empty() {
            let fallback = RecollectionCard {
                card_id: format!("hazel-fallback-{}", input.task_id),
                kind: RecollectionCardKind::SupportingContext,
                state: RecollectionCardState::Candidate,
                subject: "hazel".to_string(),
                // Keep fallback constant-sized so long goals cannot explode token usage.
                text: "No high-relevance cards found.".to_string(),
                provenance: vec![ProvenanceRef {
                    source_ref: "hazel:retrieval-fallback".to_string(),
                    source_excerpt: None,
                }],
                policy_ids: input.policy_ids.clone(),
            };
            if used_tokens.saturating_add(estimate_recollection_card_tokens(&fallback))
                <= token_budget
            {
                cards.push(fallback);
            }
        }

        RecollectionPayload {
            policy_ids: input.policy_ids.clone(),
            cards,
        }
    }
}

fn is_relevant(assertion: &Assertion, goal: &str, threshold: f32) -> bool {
    let goal = goal.to_ascii_lowercase();
    let haystack = format!(
        "{} {} {}",
        assertion.subject, assertion.predicate, assertion.object
    )
    .to_ascii_lowercase();
    let overlap = goal
        .split_whitespace()
        .filter(|token| haystack.contains(token))
        .count();
    let denom = goal.split_whitespace().count().max(1) as f32;
    (overlap as f32 / denom) >= threshold
}

fn assertion_to_card(
    assertion: &Assertion,
    hint: &HazelCardPolicyHint,
    goal: &str,
) -> RecollectionCard {
    RecollectionCard {
        card_id: assertion.assertion_id.clone(),
        kind: hint.kind.clone(),
        state: match assertion.state {
            AssertionState::Candidate => RecollectionCardState::Candidate,
            AssertionState::Active => RecollectionCardState::Active,
            AssertionState::Contested => RecollectionCardState::Contested,
            AssertionState::Deprecated => RecollectionCardState::Deprecated,
        },
        subject: assertion.subject.clone(),
        text: format!(
            "{} {} {} (goal={})",
            assertion.subject, assertion.predicate, assertion.object, goal
        ),
        provenance: vec![ProvenanceRef {
            source_ref: format!("hazel:assertion/{}", assertion.assertion_id),
            source_excerpt: Some(format!(
                "support={} contradiction={} confidence_milli={}",
                assertion.support_count, assertion.contradiction_count, assertion.confidence_milli
            )),
        }],
        policy_ids: hint.policy_ids.clone(),
    }
}
