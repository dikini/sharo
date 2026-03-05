use std::collections::BTreeMap;

use crate::model_connector::{
    ConnectorError, ModelConnectorPort, ModelProfile, ModelTurnRequest,
};
use crate::reasoning_context::{Composer, ContextState, NoOpComposer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasoningInput {
    pub trace_id: String,
    pub task_id: String,
    pub goal: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasoningOutcome {
    pub route_decision_details: String,
    pub model_output_text: String,
}

pub trait ReasoningEnginePort {
    fn plan(&self, input: &ReasoningInput) -> Result<ReasoningOutcome, String>;
}

#[derive(Debug, Clone)]
pub struct IdReasoningEngine<C: ModelConnectorPort> {
    connector: C,
    profile: ModelProfile,
}

impl<C: ModelConnectorPort> IdReasoningEngine<C> {
    pub fn new(connector: C, profile: ModelProfile) -> Self {
        Self { connector, profile }
    }
}

impl<C: ModelConnectorPort> ReasoningEnginePort for IdReasoningEngine<C> {
    fn plan(&self, input: &ReasoningInput) -> Result<ReasoningOutcome, String> {
        let context_state = ContextState::from_reasoning_input_defaults(input);
        let prompt = NoOpComposer.compose(&context_state);
        let request = ModelTurnRequest {
            trace_id: input.trace_id.clone(),
            task_id: input.task_id.clone(),
            prompt: prompt.prompt_text,
            metadata: BTreeMap::new(),
        };
        let response = self
            .connector
            .run_turn(&self.profile, &request)
            .map_err(|error| format_connector_error(&error))?;
        Ok(ReasoningOutcome {
            route_decision_details: response.route_label,
            model_output_text: response.content,
        })
    }
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
