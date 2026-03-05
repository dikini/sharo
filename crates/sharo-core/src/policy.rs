use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionClass {
    Read,
    RestrictedWrite,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyContext {
    pub task_id: String,
    pub step_id: String,
    pub action_class: ActionClass,
    pub autonomy_mode: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyDecisionKind {
    Allow,
    Deny,
    RequireApproval,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub decision: PolicyDecisionKind,
    pub reason: String,
}

#[derive(Debug, Default, Clone)]
pub struct PolicyEngine;

impl PolicyEngine {
    pub fn evaluate(&self, context: &PolicyContext) -> PolicyDecision {
        match (context.action_class, context.autonomy_mode.as_str()) {
            (ActionClass::Read, _) => PolicyDecision {
                decision: PolicyDecisionKind::Allow,
                reason: "read_allowed".to_string(),
            },
            (ActionClass::RestrictedWrite, "observe") => PolicyDecision {
                decision: PolicyDecisionKind::Deny,
                reason: "restricted_denied_in_observe".to_string(),
            },
            (ActionClass::RestrictedWrite, _) => PolicyDecision {
                decision: PolicyDecisionKind::RequireApproval,
                reason: "restricted_requires_approval".to_string(),
            },
        }
    }
}
