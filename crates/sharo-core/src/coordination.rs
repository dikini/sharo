use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoordinationIntentRecord {
    pub intent_id: String,
    pub task_id: String,
    pub scope: String,
    pub goal: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoordinationClaimRecord {
    pub claim_id: String,
    pub task_id: String,
    pub scope: String,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoordinationConflictRecord {
    pub conflict_id: String,
    pub task_id: String,
    pub related_task_id: String,
    pub scope: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoordinationChannelRecord {
    pub channel_id: String,
    pub conflict_id: String,
    pub task_id: String,
    pub related_task_id: String,
}
