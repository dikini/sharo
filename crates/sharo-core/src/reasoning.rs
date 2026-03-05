use std::collections::BTreeMap;

use crate::context_resolvers::{ResolvedContext, ResolverBundle, resolve_context};
use crate::model_connector::{
    ConnectorError, ModelConnectorPort, ModelProfile, ModelTurnRequest,
};
use crate::reasoning_context::{ContextState, TurnScope};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasoningInput {
    pub trace_id: String,
    pub task_id: String,
    pub session_id: String,
    pub turn_id: u64,
    pub goal: String,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasoningOutcome {
    pub route_decision_details: String,
    pub model_output_text: String,
    pub resolved_context: ResolvedContext,
}

pub trait ReasoningEnginePort {
    fn plan(&self, input: &ReasoningInput) -> Result<ReasoningOutcome, String>;
}

pub struct IdReasoningEngine<C: ModelConnectorPort> {
    connector: C,
    profile: ModelProfile,
    resolvers: ResolverBundle,
}

impl<C: ModelConnectorPort> IdReasoningEngine<C> {
    pub fn new(connector: C, profile: ModelProfile) -> Self {
        Self {
            connector,
            profile,
            resolvers: ResolverBundle::default(),
        }
    }

    pub fn with_resolvers(connector: C, profile: ModelProfile, resolvers: ResolverBundle) -> Self {
        Self {
            connector,
            profile,
            resolvers,
        }
    }
}

impl<C: ModelConnectorPort> ReasoningEnginePort for IdReasoningEngine<C> {
    fn plan(&self, input: &ReasoningInput) -> Result<ReasoningOutcome, String> {
        let scope = TurnScope {
            session_id: input.session_id.clone(),
            task_id: input.task_id.clone(),
            turn_id: input.turn_id,
            goal: input.goal.clone(),
        };
        let resolved_context = resolve_context(&self.resolvers, &scope)?;
        let context_state = ContextState {
            system: resolved_context.system.content.clone(),
            persona: resolved_context.persona.content.clone(),
            memory: resolved_context.memory.content.clone(),
            runtime: resolved_context.runtime.content.clone(),
            goal: input.goal.clone(),
        };
        let prompt = compose_resolved_prompt(&context_state);
        let request = ModelTurnRequest {
            trace_id: input.trace_id.clone(),
            task_id: input.task_id.clone(),
            prompt,
            metadata: input.metadata.clone(),
        };
        let response = self
            .connector
            .run_turn(&self.profile, &request)
            .map_err(|error| format_connector_error(&error))?;
        Ok(ReasoningOutcome {
            route_decision_details: response.route_label,
            model_output_text: response.content,
            resolved_context,
        })
    }
}

fn compose_resolved_prompt(state: &ContextState) -> String {
    let mut segments = Vec::new();
    if !state.system.is_empty() {
        segments.push(format!("SYSTEM:\n{}", state.system));
    }
    if !state.persona.is_empty() {
        segments.push(format!("PERSONA:\n{}", state.persona));
    }
    if !state.memory.is_empty() {
        segments.push(format!("MEMORY:\n{}", state.memory));
    }
    if !state.runtime.is_empty() {
        segments.push(format!("RUNTIME:\n{}", state.runtime));
    }
    segments.push(format!("GOAL:\n{}", state.goal));
    segments.join("\n\n")
}

fn format_connector_error(error: &ConnectorError) -> String {
    match error {
        ConnectorError::Auth(message)
        | ConnectorError::RateLimit(message)
        | ConnectorError::Quota(message)
        | ConnectorError::InvalidRequest(message)
        | ConnectorError::Timeout(message)
        | ConnectorError::Unavailable(message)
        | ConnectorError::ProtocolMismatch(message)
        | ConnectorError::Internal(message) => message.clone(),
    }
}
