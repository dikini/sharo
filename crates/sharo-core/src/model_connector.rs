use std::collections::BTreeMap;
use std::net::IpAddr;
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelCapabilityFlags {
    pub supports_tools: bool,
    pub supports_json_mode: bool,
    pub supports_streaming: bool,
    pub supports_vision: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelProfile {
    pub profile_id: String,
    pub provider_id: String,
    pub model_id: String,
    pub base_url: Option<String>,
    pub auth_env_key: Option<String>,
    pub timeout_ms: u64,
    pub max_retries: u32,
    pub capabilities: ModelCapabilityFlags,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelTurnRequest {
    pub trace_id: String,
    pub task_id: String,
    pub prompt: String,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelTurnResponse {
    pub provider_request_id: Option<String>,
    pub route_label: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectorError {
    Auth(String),
    RateLimit(String),
    Quota(String),
    InvalidRequest(String),
    Timeout(String),
    Unavailable(String),
    ProtocolMismatch(String),
    Internal(String),
}

pub trait ModelConnectorPort {
    fn run_turn(
        &self,
        _profile: &ModelProfile,
        request: &ModelTurnRequest,
    ) -> Result<ModelTurnResponse, ConnectorError>;
}

pub fn validate_base_url_security(profile: &ModelProfile) -> Result<(), ConnectorError> {
    let Some(base_url) = profile.base_url.as_deref() else {
        return Ok(());
    };

    if profile.auth_env_key.is_none() {
        return Ok(());
    }

    let parsed = Url::parse(base_url).map_err(|error| {
        ConnectorError::InvalidRequest(format!("provider_base_url_invalid error={error}"))
    })?;
    match parsed.scheme() {
        "https" => Ok(()),
        "http" if is_loopback_host(parsed.host_str()) => Ok(()),
        "http" => Err(ConnectorError::InvalidRequest(
            "provider_base_url_insecure scheme=http requires loopback host when auth_env_key is set"
                .to_string(),
        )),
        scheme => Err(ConnectorError::InvalidRequest(format!(
            "provider_base_url_unsupported scheme={scheme}"
        ))),
    }
}

fn is_loopback_host(host: Option<&str>) -> bool {
    let Some(host) = host else {
        return false;
    };
    let normalized = host.trim_start_matches('[').trim_end_matches(']');
    if normalized.eq_ignore_ascii_case("localhost") {
        return true;
    }
    if normalized
        .parse::<IpAddr>()
        .map(|ip| ip.is_loopback())
        .unwrap_or(false)
    {
        return true;
    }
    parse_legacy_decimal_ipv4(normalized)
        .map(|ip| ip.is_loopback())
        .unwrap_or(false)
}

fn parse_legacy_decimal_ipv4(host: &str) -> Option<std::net::Ipv4Addr> {
    if host.is_empty() || !host.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
        return None;
    }

    let parts: Vec<&str> = host.split('.').collect();
    if parts.is_empty() || parts.len() > 4 || parts.iter().any(|part| part.is_empty()) {
        return None;
    }

    let mut values = [0u32; 4];
    for (index, part) in parts.iter().enumerate() {
        values[index] = part.parse::<u32>().ok()?;
    }

    let addr = match parts.len() {
        1 => values[0],
        2 if values[0] <= 0xff && values[1] <= 0x00ff_ffff => (values[0] << 24) | values[1],
        3 if values[0] <= 0xff && values[1] <= 0xff && values[2] <= 0x0000_ffff => {
            (values[0] << 24) | (values[1] << 16) | values[2]
        }
        4 if values.iter().all(|value| *value <= 0xff) => {
            (values[0] << 24) | (values[1] << 16) | (values[2] << 8) | values[3]
        }
        _ => return None,
    };

    Some(std::net::Ipv4Addr::from(addr))
}

#[derive(Debug, Default, Clone)]
pub struct DeterministicConnector;

impl ModelConnectorPort for DeterministicConnector {
    fn run_turn(
        &self,
        _profile: &ModelProfile,
        request: &ModelTurnRequest,
    ) -> Result<ModelTurnResponse, ConnectorError> {
        Ok(ModelTurnResponse {
            provider_request_id: None,
            route_label: "local_mock".to_string(),
            content: format!("deterministic-response task={} prompt={}", request.task_id, request.prompt),
        })
    }
}
