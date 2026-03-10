use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Chunk {
    pub chunk_id: String,
    pub content: String,
    pub source_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entity {
    pub entity_id: String,
    pub label: String,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Relation {
    pub relation_id: String,
    pub from_entity_id: String,
    pub to_entity_id: String,
    pub relation_kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Association {
    pub association_id: String,
    pub from_entity_id: String,
    pub to_entity_id: String,
    pub coactivation_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssertionState {
    Candidate,
    Active,
    Contested,
    Deprecated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assertion {
    pub assertion_id: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub lineage: Vec<String>,
    pub support_count: u64,
    pub contradiction_count: u64,
    pub confidence_milli: u16,
    pub state: AssertionState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Activation {
    pub assertion_id: String,
    pub activation_count: u64,
    pub recency_ticks: u64,
}

impl Assertion {
    pub fn derive_from(
        base: &Assertion,
        assertion_id: impl Into<String>,
        predicate: impl Into<String>,
        object: impl Into<String>,
    ) -> Self {
        let mut lineage = base.lineage.clone();
        lineage.push(base.assertion_id.clone());
        Self {
            assertion_id: assertion_id.into(),
            subject: base.subject.clone(),
            predicate: predicate.into(),
            object: object.into(),
            lineage,
            support_count: 0,
            contradiction_count: 0,
            confidence_milli: base.confidence_milli,
            state: AssertionState::Candidate,
        }
    }
}

pub fn association_implies_relation(_association: &Association) -> bool {
    false
}
