use reqwest::blocking::Client;
use serde_json::Value;

use crate::model_connector::{
    ConnectorError, ModelConnectorPort, ModelProfile, ModelTurnRequest, ModelTurnResponse,
};

#[derive(Debug, Clone)]
pub struct OpenAiCompatibleConnector {
    client: Client,
}

impl Default for OpenAiCompatibleConnector {
    fn default() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl ModelConnectorPort for OpenAiCompatibleConnector {
    fn run_turn(
        &self,
        profile: &ModelProfile,
        request: &ModelTurnRequest,
    ) -> Result<ModelTurnResponse, ConnectorError> {
        let base_url = profile.base_url.as_deref().ok_or_else(|| {
            ConnectorError::InvalidRequest("model profile requires base_url".to_string())
        })?;

        let url = format!("{}/v1/responses", base_url.trim_end_matches('/'));
        let mut req = self.client.post(url).json(&serde_json::json!({
            "model": profile.model_id,
            "input": request.prompt,
        }));

        if let Some(env_key) = profile.auth_env_key.as_deref() {
            let token = std::env::var(env_key)
                .map_err(|_| ConnectorError::Auth(format!("missing auth env var {}", env_key)))?;
            if token.trim().is_empty() {
                return Err(ConnectorError::Auth(format!(
                    "empty auth env var {}",
                    env_key
                )));
            }
            req = req.bearer_auth(token);
        }

        let response = req.send().map_err(|e| {
            if e.is_timeout() {
                ConnectorError::Timeout(e.to_string())
            } else if e.is_connect() {
                ConnectorError::Unavailable(e.to_string())
            } else {
                ConnectorError::Internal(e.to_string())
            }
        })?;

        let status = response.status();
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(ConnectorError::RateLimit("provider rate limit".to_string()));
        }
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(ConnectorError::Auth(format!("provider auth failure status={status}")));
        }
        if !status.is_success() {
            return Err(ConnectorError::InvalidRequest(format!(
                "provider request failed status={status}"
            )));
        }

        let body: Value = response
            .json()
            .map_err(|e| ConnectorError::ProtocolMismatch(e.to_string()))?;
        let content = body
            .get("output_text")
            .and_then(Value::as_str)
            .or_else(|| {
                body.get("output")
                    .and_then(Value::as_array)
                    .and_then(|items| items.first())
                    .and_then(|item| item.get("content"))
                    .and_then(Value::as_array)
                    .and_then(|content| content.first())
                    .and_then(|chunk| chunk.get("text"))
                    .and_then(Value::as_str)
            })
            .unwrap_or_default()
            .to_string();

        Ok(ModelTurnResponse {
            provider_request_id: body.get("id").and_then(Value::as_str).map(|v| v.to_string()),
            route_label: format!("{}:{}", profile.provider_id, profile.model_id),
            content,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct OllamaConnector {
    openai_compat: OpenAiCompatibleConnector,
}

impl ModelConnectorPort for OllamaConnector {
    fn run_turn(
        &self,
        profile: &ModelProfile,
        request: &ModelTurnRequest,
    ) -> Result<ModelTurnResponse, ConnectorError> {
        self.openai_compat.run_turn(profile, request)
    }
}
