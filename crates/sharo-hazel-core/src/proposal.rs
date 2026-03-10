use serde::{Deserialize, Serialize};

use crate::domain::{Assertion, Chunk, Entity, Relation};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalKind {
    ChunkUpsert,
    EntityUpsert,
    RelationUpsert,
    AssertionUpsert,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    pub proposal_id: String,
    pub kind: ProposalKind,
    pub chunk: Option<Chunk>,
    pub entity: Option<Entity>,
    pub relation: Option<Relation>,
    pub assertion: Option<Assertion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BatchProvenance {
    pub source_ref: String,
    pub producer: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposalBatch {
    pub batch_id: String,
    pub idempotency_key: String,
    pub provenance: BatchProvenance,
    pub proposals: Vec<Proposal>,
}
