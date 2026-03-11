use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpTransportKind {
    Stdio,
    Http,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpRuntimeStatus {
    Disabled,
    Configured,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpServerSummary {
    pub server_id: String,
    pub display_name: String,
    pub transport_kind: McpTransportKind,
    pub enabled: bool,
    pub runtime_status: McpRuntimeStatus,
    pub startup_timeout_ms: Option<u64>,
    pub trust_class: String,
    pub diagnostic_summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeStatusSummary {
    pub daemon_ready: bool,
    pub config_loaded: bool,
    pub model_profile_id: Option<String>,
    pub mcp_enabled_count: usize,
    pub mcp_disabled_count: usize,
    pub warnings: Vec<String>,
}
