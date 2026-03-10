use crate::domain::{Assertion, AssertionState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionDecision {
    pub from: AssertionState,
    pub to: AssertionState,
    pub score_milli: i32,
}

pub fn deterministic_score(assertion: &Assertion) -> i32 {
    let support = (assertion.support_count as i32) * 100;
    let contradictions = (assertion.contradiction_count as i32) * 130;
    support - contradictions + i32::from(assertion.confidence_milli)
}

pub fn resolve_state(assertion: &Assertion) -> TransitionDecision {
    let score = deterministic_score(assertion);
    let to = if score >= 1_000 {
        AssertionState::Active
    } else if score <= -100 {
        AssertionState::Contested
    } else {
        AssertionState::Candidate
    };
    TransitionDecision {
        from: assertion.state.clone(),
        to,
        score_milli: score,
    }
}
