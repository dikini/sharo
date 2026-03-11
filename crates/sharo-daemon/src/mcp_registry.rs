use std::collections::{BTreeMap, BTreeSet};

use sharo_core::mcp::{McpRuntimeStatus, McpServerSummary, McpTransportKind, RuntimeStatusSummary};
use url::{Host, Url};

use crate::config::{McpConfig, McpServerConfig, ModelRuntimeConfig};

const MAX_RUNTIME_WARNINGS: usize = 32;
const MAX_MCP_SERVERS: usize = 128;
const MAX_DIAGNOSTIC_SUMMARY_CHARS: usize = 256;

#[derive(Debug, Clone)]
struct McpServerRecord {
    server_id: String,
    display_name: String,
    transport_kind: McpTransportKind,
    startup_timeout_ms: Option<u64>,
    trust_class: String,
    default_enabled: bool,
    diagnostic_summary: Option<String>,
}

#[derive(Debug, Clone)]
pub struct McpRegistry {
    servers: BTreeMap<String, McpServerRecord>,
}

impl McpRegistry {
    pub fn from_config(config: &McpConfig) -> Result<Self, String> {
        validate_mcp_config(config)?;
        let mut servers = BTreeMap::new();
        for server in &config.servers {
            let record = McpServerRecord {
                server_id: server.server_id.clone(),
                display_name: server
                    .display_name
                    .clone()
                    .unwrap_or_else(|| server.server_id.clone()),
                transport_kind: server.transport,
                startup_timeout_ms: server.startup_timeout_ms,
                trust_class: server
                    .trust_class
                    .clone()
                    .unwrap_or_else(|| "operator_configured".to_string()),
                default_enabled: server.enabled.unwrap_or(true),
                diagnostic_summary: Some(truncate_summary(match server.transport {
                    McpTransportKind::Stdio => {
                        format!("stdio command={}", server.command.as_deref().unwrap_or(""))
                    }
                    McpTransportKind::Http => {
                        format!("http endpoint={}", server.endpoint.as_deref().unwrap_or(""))
                    }
                })),
            };
            servers.insert(record.server_id.clone(), record);
        }
        Ok(Self { servers })
    }

    pub fn list_servers(
        &self,
        enabled_overrides: &BTreeMap<String, bool>,
    ) -> Vec<McpServerSummary> {
        self.servers
            .values()
            .map(|server| self.render_summary(server, enabled_overrides.get(&server.server_id)))
            .collect()
    }

    pub fn get_server(
        &self,
        server_id: &str,
        enabled_override: Option<bool>,
    ) -> Option<McpServerSummary> {
        self.servers
            .get(server_id)
            .map(|server| self.render_summary(server, enabled_override.as_ref()))
    }

    pub fn contains_server(&self, server_id: &str) -> bool {
        self.servers.contains_key(server_id)
    }

    pub fn server_ids(&self) -> BTreeSet<String> {
        self.servers.keys().cloned().collect()
    }

    pub fn runtime_status(
        &self,
        enabled_overrides: &BTreeMap<String, bool>,
        model: &ModelRuntimeConfig,
    ) -> RuntimeStatusSummary {
        let servers = self.list_servers(enabled_overrides);
        let mcp_enabled_count = servers.iter().filter(|server| server.enabled).count();
        let warnings = servers
            .iter()
            .filter(|server| {
                server.enabled && server.runtime_status != McpRuntimeStatus::Configured
            })
            .map(|server| format!("mcp_server_not_configured server_id={}", server.server_id))
            .take(MAX_RUNTIME_WARNINGS)
            .collect::<Vec<_>>();
        RuntimeStatusSummary {
            daemon_ready: true,
            config_loaded: true,
            model_profile_id: model.profile_id.clone(),
            mcp_enabled_count,
            mcp_disabled_count: servers.len().saturating_sub(mcp_enabled_count),
            warnings,
        }
    }

    fn render_summary(
        &self,
        server: &McpServerRecord,
        enabled_override: Option<&bool>,
    ) -> McpServerSummary {
        let enabled = enabled_override.copied().unwrap_or(server.default_enabled);
        McpServerSummary {
            server_id: server.server_id.clone(),
            display_name: server.display_name.clone(),
            transport_kind: server.transport_kind,
            enabled,
            runtime_status: if enabled {
                McpRuntimeStatus::Configured
            } else {
                McpRuntimeStatus::Disabled
            },
            startup_timeout_ms: server.startup_timeout_ms,
            trust_class: server.trust_class.clone(),
            diagnostic_summary: server.diagnostic_summary.clone(),
        }
    }
}

pub fn validate_mcp_config(config: &McpConfig) -> Result<(), String> {
    if config.servers.len() > MAX_MCP_SERVERS {
        return Err(format!(
            "mcp_config_invalid too_many_servers max={} actual={}",
            MAX_MCP_SERVERS,
            config.servers.len()
        ));
    }
    let mut seen_server_ids = BTreeSet::new();
    for server in &config.servers {
        if server.server_id.trim().is_empty() {
            return Err("mcp_config_invalid empty_server_id".to_string());
        }
        if !seen_server_ids.insert(server.server_id.clone()) {
            return Err(format!(
                "mcp_config_invalid duplicate_server_id server_id={}",
                server.server_id
            ));
        }
        validate_server_shape(server)?;
    }
    Ok(())
}

fn truncate_summary(summary: String) -> String {
    if summary.chars().count() <= MAX_DIAGNOSTIC_SUMMARY_CHARS {
        return summary;
    }
    summary
        .chars()
        .take(MAX_DIAGNOSTIC_SUMMARY_CHARS)
        .collect::<String>()
}

fn validate_server_shape(server: &McpServerConfig) -> Result<(), String> {
    if server.startup_timeout_ms.is_some_and(|value| value == 0) {
        return Err(format!(
            "mcp_config_invalid startup_timeout_ms_zero server_id={}",
            server.server_id
        ));
    }

    match server.transport {
        McpTransportKind::Stdio => {
            if server.command.as_deref().is_none_or(str::is_empty) {
                return Err(format!(
                    "mcp_config_invalid stdio_command_missing server_id={}",
                    server.server_id
                ));
            }
            if server.endpoint.is_some() {
                return Err(format!(
                    "mcp_config_invalid stdio_endpoint_forbidden server_id={}",
                    server.server_id
                ));
            }
        }
        McpTransportKind::Http => {
            let endpoint = server.endpoint.as_deref().ok_or_else(|| {
                format!(
                    "mcp_config_invalid http_endpoint_missing server_id={}",
                    server.server_id
                )
            })?;
            if endpoint.is_empty() {
                return Err(format!(
                    "mcp_config_invalid http_endpoint_missing server_id={}",
                    server.server_id
                ));
            }
            validate_http_endpoint(server.server_id.as_str(), endpoint)?;
            if server.command.is_some() || server.args.as_ref().is_some_and(|args| !args.is_empty())
            {
                return Err(format!(
                    "mcp_config_invalid http_command_forbidden server_id={}",
                    server.server_id
                ));
            }
        }
    }

    Ok(())
}

fn validate_http_endpoint(server_id: &str, endpoint: &str) -> Result<(), String> {
    let parsed = Url::parse(endpoint).map_err(|error| {
        format!(
            "mcp_config_invalid http_endpoint_parse_failed server_id={} error={}",
            server_id, error
        )
    })?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(format!(
            "mcp_config_invalid http_endpoint_scheme_forbidden server_id={} scheme={}",
            server_id,
            parsed.scheme()
        ));
    }
    if parsed.host_str().is_none() {
        return Err(format!(
            "mcp_config_invalid http_endpoint_host_missing server_id={server_id}"
        ));
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(format!(
            "mcp_config_invalid http_endpoint_credentials_forbidden server_id={server_id}"
        ));
    }
    if parsed.query().is_some() {
        return Err(format!(
            "mcp_config_invalid http_endpoint_query_forbidden server_id={server_id}"
        ));
    }
    if parsed.fragment().is_some() {
        return Err(format!(
            "mcp_config_invalid http_endpoint_fragment_forbidden server_id={server_id}"
        ));
    }
    if parsed.scheme() == "http" && !is_loopback_host(parsed.host()) {
        return Err(format!(
            "mcp_config_invalid http_endpoint_cleartext_remote_forbidden server_id={server_id}"
        ));
    }
    Ok(())
}

fn is_loopback_host(host: Option<Host<&str>>) -> bool {
    match host {
        Some(Host::Domain(domain)) => domain.eq_ignore_ascii_case("localhost"),
        Some(Host::Ipv4(address)) => address.is_loopback(),
        Some(Host::Ipv6(address)) => address.is_loopback(),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use proptest::prelude::*;
    use sharo_core::mcp::{McpRuntimeStatus, McpTransportKind};

    use crate::config::{DaemonConfigFile, McpConfig, McpServerConfig, ModelRuntimeConfig};

    use super::{McpRegistry, validate_mcp_config};

    #[test]
    fn mcp_registry_shapes_server_status_summary() {
        let registry = McpRegistry::from_config(&McpConfig {
            servers: vec![McpServerConfig {
                server_id: "hazel".to_string(),
                display_name: Some("Hazel".to_string()),
                transport: McpTransportKind::Stdio,
                command: Some("/usr/bin/hazel-mcp".to_string()),
                args: Some(vec!["--stdio".to_string()]),
                endpoint: None,
                startup_timeout_ms: Some(250),
                trust_class: Some("operator".to_string()),
                enabled: Some(true),
            }],
        })
        .expect("registry");

        let servers = registry.list_servers(&BTreeMap::new());
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].server_id, "hazel");
        assert_eq!(servers[0].display_name, "Hazel");
        assert_eq!(servers[0].transport_kind, McpTransportKind::Stdio);
        assert_eq!(servers[0].runtime_status, McpRuntimeStatus::Configured);
        assert_eq!(servers[0].startup_timeout_ms, Some(250));
        assert_eq!(servers[0].trust_class, "operator");
    }

    #[test]
    fn disabled_mcp_server_never_reports_running_status() {
        let registry = McpRegistry::from_config(&McpConfig {
            servers: vec![McpServerConfig {
                server_id: "docs".to_string(),
                display_name: None,
                transport: McpTransportKind::Http,
                command: None,
                args: None,
                endpoint: Some("http://127.0.0.1:4000/mcp".to_string()),
                startup_timeout_ms: None,
                trust_class: None,
                enabled: Some(false),
            }],
        })
        .expect("registry");

        let servers = registry.list_servers(&BTreeMap::new());
        assert_eq!(servers[0].runtime_status, McpRuntimeStatus::Disabled);
        assert!(!servers[0].enabled);
    }

    #[test]
    fn runtime_status_counts_enabled_and_disabled_servers() {
        let registry = McpRegistry::from_config(&McpConfig {
            servers: vec![
                McpServerConfig {
                    server_id: "hazel".to_string(),
                    display_name: None,
                    transport: McpTransportKind::Stdio,
                    command: Some("/usr/bin/hazel".to_string()),
                    args: None,
                    endpoint: None,
                    startup_timeout_ms: None,
                    trust_class: None,
                    enabled: Some(true),
                },
                McpServerConfig {
                    server_id: "docs".to_string(),
                    display_name: None,
                    transport: McpTransportKind::Http,
                    command: None,
                    args: None,
                    endpoint: Some("http://127.0.0.1:4000/mcp".to_string()),
                    startup_timeout_ms: None,
                    trust_class: None,
                    enabled: Some(false),
                },
            ],
        })
        .expect("registry");

        let runtime = registry.runtime_status(&BTreeMap::new(), &ModelRuntimeConfig::default());
        assert!(runtime.daemon_ready);
        assert!(runtime.config_loaded);
        assert_eq!(runtime.mcp_enabled_count, 1);
        assert_eq!(runtime.mcp_disabled_count, 1);
    }

    #[test]
    fn http_endpoint_rejects_credentials_and_query_components() {
        let with_credentials = McpConfig {
            servers: vec![McpServerConfig {
                server_id: "docs".to_string(),
                display_name: None,
                transport: McpTransportKind::Http,
                command: None,
                args: None,
                endpoint: Some("https://user:secret@example.com/mcp".to_string()),
                startup_timeout_ms: None,
                trust_class: None,
                enabled: Some(true),
            }],
        };
        let error = validate_mcp_config(&with_credentials).expect_err("invalid credentials");
        assert!(error.contains("http_endpoint_credentials_forbidden"));

        let with_query = McpConfig {
            servers: vec![McpServerConfig {
                server_id: "docs".to_string(),
                display_name: None,
                transport: McpTransportKind::Http,
                command: None,
                args: None,
                endpoint: Some("https://example.com/mcp?token=secret".to_string()),
                startup_timeout_ms: None,
                trust_class: None,
                enabled: Some(true),
            }],
        };
        let error = validate_mcp_config(&with_query).expect_err("invalid query");
        assert!(error.contains("http_endpoint_query_forbidden"));
    }

    #[test]
    fn http_endpoint_rejects_cleartext_remote_hosts() {
        let config = McpConfig {
            servers: vec![McpServerConfig {
                server_id: "docs".to_string(),
                display_name: None,
                transport: McpTransportKind::Http,
                command: None,
                args: None,
                endpoint: Some("http://example.com/mcp".to_string()),
                startup_timeout_ms: None,
                trust_class: None,
                enabled: Some(true),
            }],
        };
        let error = validate_mcp_config(&config).expect_err("invalid remote http");
        assert!(error.contains("http_endpoint_cleartext_remote_forbidden"));
    }

    proptest! {
        #[test]
        fn mcp_server_ids_are_unique_after_valid_config_parse(
            ids in proptest::collection::btree_set("[a-z][a-z0-9_-]{0,7}", 1..8)
        ) {
            let mut raw = String::new();
            for id in ids.iter() {
                raw.push_str(&format!(
                    "[[mcp.servers]]\nserver_id = \"{id}\"\ntransport = \"stdio\"\ncommand = \"/usr/bin/{id}\"\n"
                ));
            }
            let parsed: DaemonConfigFile = toml::from_str(&raw).expect("parse");
            validate_mcp_config(&parsed.mcp).expect("valid config");
            let registry = McpRegistry::from_config(&parsed.mcp).expect("registry");
            let listed_ids = registry
                .list_servers(&BTreeMap::new())
                .into_iter()
                .map(|server| server.server_id)
                .collect::<BTreeSet<_>>();
            prop_assert_eq!(listed_ids.len(), ids.len());
            prop_assert_eq!(listed_ids, ids);
        }
    }
}
