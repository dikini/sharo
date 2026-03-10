use reqwest::blocking::Client;
use serde_json::Value;
use std::sync::OnceLock;
use std::time::Duration;

use crate::model_connector::{
    ConnectorError, ModelConnectorPort, ModelProfile, ModelTurnRequest, ModelTurnResponse,
    validate_base_url_security,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct OpenAiCompatibleConnector;

fn shared_blocking_client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(Client::new)
}

impl ModelConnectorPort for OpenAiCompatibleConnector {
    fn run_turn(
        &self,
        profile: &ModelProfile,
        request: &ModelTurnRequest,
    ) -> Result<ModelTurnResponse, ConnectorError> {
        if profile.timeout_ms == 0 {
            return Err(ConnectorError::InvalidRequest(
                "model profile timeout_ms must be > 0".to_string(),
            ));
        }
        let base_url = profile.base_url.as_deref().ok_or_else(|| {
            ConnectorError::InvalidRequest("model profile requires base_url".to_string())
        })?;
        validate_base_url_security(profile)?;

        let url = format!("{}/v1/responses", base_url.trim_end_matches('/'));
        let mut req = shared_blocking_client()
            .post(url)
            .timeout(Duration::from_millis(profile.timeout_ms))
            .json(&serde_json::json!({
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

        let response = req.send().map_err(classify_transport_error)?;

        let status = response.status();
        if !status.is_success() {
            return Err(classify_http_status(status));
        }

        let body: Value = response
            .json()
            .map_err(|e| ConnectorError::ProtocolMismatch(e.to_string()))?;
        let content = extract_output_text(&body)?;

        Ok(ModelTurnResponse {
            provider_request_id: body
                .get("id")
                .and_then(Value::as_str)
                .map(|v| v.to_string()),
            route_label: format!("{}:{}", profile.provider_id, profile.model_id),
            content,
        })
    }
}

fn classify_transport_error(error: reqwest::Error) -> ConnectorError {
    if error.is_timeout() {
        ConnectorError::Timeout(error.to_string())
    } else if error.is_connect() {
        ConnectorError::Unavailable(error.to_string())
    } else {
        ConnectorError::Internal(error.to_string())
    }
}

fn classify_http_status(status: reqwest::StatusCode) -> ConnectorError {
    match status {
        reqwest::StatusCode::REQUEST_TIMEOUT | reqwest::StatusCode::GATEWAY_TIMEOUT => {
            ConnectorError::Timeout(format!("provider timeout status={status}"))
        }
        reqwest::StatusCode::TOO_MANY_REQUESTS => {
            ConnectorError::RateLimit(format!("provider rate limit status={status}"))
        }
        reqwest::StatusCode::PAYMENT_REQUIRED => {
            ConnectorError::Quota(format!("provider quota exceeded status={status}"))
        }
        reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN => {
            ConnectorError::Auth(format!("provider auth failure status={status}"))
        }
        status if status.is_server_error() => {
            ConnectorError::Unavailable(format!("provider unavailable status={status}"))
        }
        _ => ConnectorError::InvalidRequest(format!("provider request failed status={status}")),
    }
}

fn extract_output_text(body: &Value) -> Result<String, ConnectorError> {
    if let Some(text) = body.get("output_text").and_then(Value::as_str)
        && !text.trim().is_empty()
    {
        return Ok(text.to_string());
    }

    let mut collected = String::new();
    if let Some(items) = body.get("output").and_then(Value::as_array) {
        for item in items {
            let Some(content) = item.get("content").and_then(Value::as_array) else {
                continue;
            };
            for chunk in content {
                let Some(text) = chunk.get("text").and_then(Value::as_str) else {
                    continue;
                };
                if !text.trim().is_empty() {
                    if !collected.is_empty() {
                        collected.push('\n');
                    }
                    collected.push_str(text);
                }
            }
        }
    }

    if !collected.is_empty() {
        return Ok(collected);
    }

    Err(ConnectorError::ProtocolMismatch(
        "provider response contained no parseable output text".to_string(),
    ))
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

#[cfg(test)]
mod tests {
    use crate::model_connector::ConnectorError;

    use super::{classify_http_status, extract_output_text};

    #[test]
    fn extract_output_text_accepts_output_text_field() {
        let body = serde_json::json!({
            "id": "resp-1",
            "output_text": "ok"
        });
        let parsed = extract_output_text(&body).expect("output_text should parse");
        assert_eq!(parsed, "ok");
    }

    #[test]
    fn extract_output_text_rejects_missing_text() {
        let body = serde_json::json!({
            "id": "resp-1",
            "output": []
        });
        let error = extract_output_text(&body).expect_err("missing text should fail");
        assert!(matches!(error, ConnectorError::ProtocolMismatch(_)));
    }

    #[test]
    fn extract_output_text_accepts_later_output_chunk_text() {
        let body = serde_json::json!({
            "id": "resp-1",
            "output": [
                {
                    "type": "reasoning",
                    "content": [
                        {"type": "reasoning", "summary": "thinking"}
                    ]
                },
                {
                    "type": "message",
                    "content": [
                        {"type": "output_text", "text": ""},
                        {"type": "output_text", "text": "final answer"}
                    ]
                }
            ]
        });
        let parsed = extract_output_text(&body).expect("later text chunk should parse");
        assert_eq!(parsed, "final answer");
    }

    #[test]
    fn extract_output_text_joins_multiple_text_chunks() {
        let body = serde_json::json!({
            "id": "resp-1",
            "output": [
                {
                    "type": "message",
                    "content": [
                        {"type": "output_text", "text": "line one"},
                        {"type": "output_text", "text": "line two"}
                    ]
                }
            ]
        });
        let parsed = extract_output_text(&body).expect("multiple chunks should parse");
        assert_eq!(parsed, "line one\nline two");
    }

    #[test]
    fn http_500_maps_to_unavailable() {
        let error = classify_http_status(reqwest::StatusCode::INTERNAL_SERVER_ERROR);
        assert!(matches!(error, ConnectorError::Unavailable(_)));
    }

    #[test]
    fn http_408_maps_to_timeout() {
        let error = classify_http_status(reqwest::StatusCode::REQUEST_TIMEOUT);
        assert!(matches!(error, ConnectorError::Timeout(_)));
    }

    #[test]
    fn http_429_maps_to_rate_limit() {
        let error = classify_http_status(reqwest::StatusCode::TOO_MANY_REQUESTS);
        assert!(matches!(error, ConnectorError::RateLimit(_)));
    }

    #[test]
    fn http_402_maps_to_quota() {
        let error = classify_http_status(reqwest::StatusCode::PAYMENT_REQUIRED);
        assert!(matches!(error, ConnectorError::Quota(_)));
    }

    #[test]
    fn http_400_maps_to_invalid_request() {
        let error = classify_http_status(reqwest::StatusCode::BAD_REQUEST);
        assert!(matches!(error, ConnectorError::InvalidRequest(_)));
    }

    #[test]
    fn non_success_statuses_never_default_retryable_codes_to_invalid_request() {
        for status in [
            reqwest::StatusCode::REQUEST_TIMEOUT,
            reqwest::StatusCode::TOO_MANY_REQUESTS,
            reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            reqwest::StatusCode::BAD_GATEWAY,
            reqwest::StatusCode::SERVICE_UNAVAILABLE,
            reqwest::StatusCode::GATEWAY_TIMEOUT,
        ] {
            let error = classify_http_status(status);
            assert!(
                !matches!(error, ConnectorError::InvalidRequest(_)),
                "retryable status {status} mapped to invalid request"
            );
        }
    }
}
