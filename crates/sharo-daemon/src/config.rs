use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sharo_core::mcp::McpTransportKind;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DaemonConfigFile {
    #[serde(default)]
    pub model: ModelRuntimeConfig,
    #[serde(default)]
    pub connector_pool: ConnectorPoolConfig,
    #[serde(default)]
    pub skills: SkillsConfig,
    #[serde(default)]
    pub mcp: McpConfig,
    #[serde(default)]
    pub reasoning_policy: ReasoningPolicyConfig,
    #[serde(default)]
    pub reasoning_context: ReasoningContextConfig,
    #[serde(default)]
    pub reasoning_hooks: ReasoningHooksConfig,
    #[serde(default)]
    pub hook_policies: BTreeMap<String, HookPolicyDefinitionConfig>,
    #[serde(default)]
    pub hazel_manifest: HazelManifestConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConnectorPoolConfig {
    pub min_threads: usize,
    pub max_threads: usize,
    pub queue_capacity: usize,
    pub scale_up_queue_threshold: usize,
    pub scale_down_idle_ms: u64,
    pub cooldown_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelRuntimeConfig {
    pub provider: Option<String>,
    pub model_id: Option<String>,
    pub base_url: Option<String>,
    pub auth_env_key: Option<String>,
    pub timeout_ms: Option<u64>,
    pub max_retries: Option<u32>,
    pub profile_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SkillsConfig {
    pub project_root: Option<String>,
    pub user_root: Option<String>,
    #[serde(default)]
    pub roots: Vec<String>,
    pub max_depth: Option<usize>,
    pub enable_project_skills: Option<bool>,
    pub enable_user_skills: Option<bool>,
    pub trust_project_skills: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct McpConfig {
    #[serde(default)]
    pub servers: Vec<McpServerConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct McpServerConfig {
    pub server_id: String,
    pub display_name: Option<String>,
    pub transport: McpTransportKind,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub endpoint: Option<String>,
    pub startup_timeout_ms: Option<u64>,
    pub trust_class: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ReasoningPolicyConfig {
    pub max_prompt_chars: Option<usize>,
    pub max_memory_lines: Option<usize>,
    pub forbidden_runtime_fields: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ReasoningContextConfig {
    pub system: Option<String>,
    pub persona: Option<String>,
    pub memory: Option<String>,
    pub runtime: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ReasoningHooksConfig {
    #[serde(default)]
    pub pre_prompt_compose: PrePromptComposeHookConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PrePromptComposeHookConfig {
    pub composition: Option<String>,
    pub bindings: Option<Vec<HookBindingConfig>>,
    pub default_policy_ids: Option<Vec<String>>,
    pub strict_unknown_policy_ids: Option<bool>,
    pub top_k: Option<usize>,
    pub token_budget: Option<usize>,
    pub relevance_threshold: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct HookBindingConfig {
    pub id: String,
    pub tool: String,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct HookPolicyDefinitionConfig {
    #[serde(default)]
    pub rules: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct HazelManifestConfig {
    #[serde(default)]
    pub cards: Vec<HazelCardManifestConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct HazelCardManifestConfig {
    pub kind: String,
    #[serde(default)]
    pub policy_ids: Vec<String>,
    pub max_cards: Option<usize>,
}

fn daemon_config_path_from_home(home: &Path) -> PathBuf {
    home.join(".config").join("sharo").join("daemon.toml")
}

pub fn default_daemon_config_path() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| daemon_config_path_from_home(&home))
}

impl Default for ModelRuntimeConfig {
    fn default() -> Self {
        Self {
            provider: Some("deterministic".to_string()),
            model_id: Some("mock".to_string()),
            base_url: None,
            auth_env_key: None,
            timeout_ms: Some(1_000),
            max_retries: Some(0),
            profile_id: Some("id-default".to_string()),
        }
    }
}

impl Default for ConnectorPoolConfig {
    fn default() -> Self {
        Self {
            min_threads: 2,
            max_threads: 4,
            queue_capacity: 64,
            scale_up_queue_threshold: 4,
            scale_down_idle_ms: 5000,
            cooldown_ms: 250,
        }
    }
}

pub fn load_daemon_config(path: Option<&Path>) -> Result<DaemonConfigFile, String> {
    let Some(path) = path else {
        return Ok(DaemonConfigFile::default());
    };
    if !path.exists() {
        return Ok(DaemonConfigFile::default());
    }

    let raw = fs::read_to_string(path).map_err(|e| {
        format!(
            "daemon_config_read_failed path={} error={}",
            path.display(),
            e
        )
    })?;
    toml::from_str::<DaemonConfigFile>(&raw).map_err(|e| {
        format!(
            "daemon_config_parse_failed path={} error={}",
            path.display(),
            e
        )
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        DaemonConfigFile, HookBindingConfig, McpTransportKind, daemon_config_path_from_home,
        default_daemon_config_path, load_daemon_config,
    };

    #[test]
    fn missing_config_path_defaults_to_deterministic_model() {
        let cfg = load_daemon_config(None).expect("load config");
        assert_eq!(cfg.model.provider.as_deref(), Some("deterministic"));
    }

    #[test]
    fn parse_model_config_from_toml() {
        let raw = r#"
[model]
provider = "openai"
model_id = "gpt-5-mini"
base_url = "https://api.openai.com"
auth_env_key = "OPENAI_API_KEY"
timeout_ms = 5000
max_retries = 2
profile_id = "openai-main"
"#;
        let parsed: DaemonConfigFile = toml::from_str(raw).expect("parse");
        assert_eq!(parsed.model.provider.as_deref(), Some("openai"));
        assert_eq!(parsed.model.model_id.as_deref(), Some("gpt-5-mini"));
        assert_eq!(
            parsed.model.base_url.as_deref(),
            Some("https://api.openai.com")
        );
        assert_eq!(parsed.model.auth_env_key.as_deref(), Some("OPENAI_API_KEY"));
        assert_eq!(parsed.model.timeout_ms, Some(5000));
        assert_eq!(parsed.model.max_retries, Some(2));
        assert_eq!(parsed.model.profile_id.as_deref(), Some("openai-main"));
    }

    #[test]
    fn parse_connector_pool_policy_from_toml() {
        let raw = r#"
[connector_pool]
min_threads = 3
max_threads = 7
queue_capacity = 128
scale_up_queue_threshold = 6
scale_down_idle_ms = 2000
cooldown_ms = 500
"#;
        let parsed: DaemonConfigFile = toml::from_str(raw).expect("parse");
        assert_eq!(parsed.connector_pool.min_threads, 3);
        assert_eq!(parsed.connector_pool.max_threads, 7);
        assert_eq!(parsed.connector_pool.queue_capacity, 128);
        assert_eq!(parsed.connector_pool.scale_up_queue_threshold, 6);
        assert_eq!(parsed.connector_pool.scale_down_idle_ms, 2000);
        assert_eq!(parsed.connector_pool.cooldown_ms, 500);
    }

    #[test]
    fn parse_reasoning_policy_and_context_from_toml() {
        let raw = r#"
[reasoning_policy]
max_prompt_chars = 256
max_memory_lines = 2
forbidden_runtime_fields = ["secret", "token"]

[reasoning_context]
system = "keep-safe"
persona = "verbosity=high"
memory = "m1\nm2\nm3"
runtime = "secret=abc123"
"#;
        let parsed: DaemonConfigFile = toml::from_str(raw).expect("parse");
        assert_eq!(parsed.reasoning_policy.max_prompt_chars, Some(256));
        assert_eq!(parsed.reasoning_policy.max_memory_lines, Some(2));
        assert_eq!(
            parsed.reasoning_policy.forbidden_runtime_fields,
            Some(vec!["secret".to_string(), "token".to_string()])
        );
        assert_eq!(
            parsed.reasoning_context.persona.as_deref(),
            Some("verbosity=high")
        );
        assert_eq!(
            parsed.reasoning_context.runtime.as_deref(),
            Some("secret=abc123")
        );
    }

    #[test]
    fn parse_reasoning_hook_and_policy_registry_from_toml() {
        let raw = r#"
[reasoning_hooks.pre_prompt_compose]
composition = "single"
default_policy_ids = ["hunch.v1"]
strict_unknown_policy_ids = true
top_k = 4
token_budget = 600
relevance_threshold = 0.2

[[reasoning_hooks.pre_prompt_compose.bindings]]
id = "hazel"
tool = "hazel.recollect"
command = "/usr/bin/hazel-mcp"
args = ["--stdio"]
timeout_ms = 250

[hook_policies."hunch.v1"]
rules = ["label_guesses", "prefer_supported_facts"]

[[hazel_manifest.cards]]
kind = "association_cue"
policy_ids = ["hunch.v1"]
max_cards = 3
"#;
        let parsed: DaemonConfigFile = toml::from_str(raw).expect("parse");
        assert_eq!(
            parsed
                .reasoning_hooks
                .pre_prompt_compose
                .composition
                .as_deref(),
            Some("single")
        );
        assert_eq!(
            parsed
                .reasoning_hooks
                .pre_prompt_compose
                .default_policy_ids
                .as_deref(),
            Some(&["hunch.v1".to_string()][..])
        );
        assert_eq!(parsed.reasoning_hooks.pre_prompt_compose.top_k, Some(4));
        assert_eq!(
            parsed.reasoning_hooks.pre_prompt_compose.token_budget,
            Some(600)
        );
        assert_eq!(
            parsed
                .reasoning_hooks
                .pre_prompt_compose
                .relevance_threshold,
            Some(0.2)
        );
        assert_eq!(
            parsed
                .reasoning_hooks
                .pre_prompt_compose
                .bindings
                .as_deref(),
            Some(
                &[HookBindingConfig {
                    id: "hazel".to_string(),
                    tool: "hazel.recollect".to_string(),
                    command: Some("/usr/bin/hazel-mcp".to_string()),
                    args: Some(vec!["--stdio".to_string()]),
                    timeout_ms: Some(250)
                }][..]
            )
        );
        assert_eq!(
            parsed
                .hook_policies
                .get("hunch.v1")
                .map(|p| p.rules.clone()),
            Some(vec![
                "label_guesses".to_string(),
                "prefer_supported_facts".to_string()
            ])
        );
        assert_eq!(parsed.hazel_manifest.cards.len(), 1);
        assert_eq!(parsed.hazel_manifest.cards[0].kind, "association_cue");
        assert_eq!(
            parsed.hazel_manifest.cards[0].policy_ids,
            vec!["hunch.v1".to_string()]
        );
        assert_eq!(parsed.hazel_manifest.cards[0].max_cards, Some(3));
    }

    #[test]
    fn parse_skills_config_from_toml() {
        let raw = r#"
[skills]
project_root = "/repo/.agents/skills"
user_root = "/home/example/.agents/skills"
roots = ["/opt/team-skills"]
max_depth = 5
enable_project_skills = true
enable_user_skills = false
trust_project_skills = true
"#;
        let parsed: DaemonConfigFile = toml::from_str(raw).expect("parse");
        assert_eq!(
            parsed.skills.project_root.as_deref(),
            Some("/repo/.agents/skills")
        );
        assert_eq!(
            parsed.skills.user_root.as_deref(),
            Some("/home/example/.agents/skills")
        );
        assert_eq!(parsed.skills.roots, vec!["/opt/team-skills".to_string()]);
        assert_eq!(parsed.skills.max_depth, Some(5));
        assert_eq!(parsed.skills.enable_project_skills, Some(true));
        assert_eq!(parsed.skills.enable_user_skills, Some(false));
        assert_eq!(parsed.skills.trust_project_skills, Some(true));
    }

    #[test]
    fn parse_mcp_config_from_toml() {
        let raw = r#"
[[mcp.servers]]
server_id = "hazel"
display_name = "Hazel"
transport = "stdio"
command = "/usr/bin/hazel-mcp"
args = ["--stdio"]
startup_timeout_ms = 250
trust_class = "operator"
enabled = true

[[mcp.servers]]
server_id = "docs"
transport = "http"
endpoint = "http://127.0.0.1:8080/mcp"
enabled = false
"#;
        let parsed: DaemonConfigFile = toml::from_str(raw).expect("parse");
        assert_eq!(parsed.mcp.servers.len(), 2);
        assert_eq!(parsed.mcp.servers[0].server_id, "hazel");
        assert_eq!(parsed.mcp.servers[0].transport, McpTransportKind::Stdio);
        assert_eq!(
            parsed.mcp.servers[0].command.as_deref(),
            Some("/usr/bin/hazel-mcp")
        );
        assert_eq!(parsed.mcp.servers[1].transport, McpTransportKind::Http);
        assert_eq!(
            parsed.mcp.servers[1].endpoint.as_deref(),
            Some("http://127.0.0.1:8080/mcp")
        );
        assert_eq!(parsed.mcp.servers[1].enabled, Some(false));
    }

    #[test]
    fn default_policy_values_are_nonzero_and_bounded() {
        let cfg = DaemonConfigFile::default();
        assert!(cfg.connector_pool.min_threads > 0);
        assert!(cfg.connector_pool.max_threads >= cfg.connector_pool.min_threads);
        assert!(cfg.connector_pool.queue_capacity > 0);
        assert!(cfg.connector_pool.scale_up_queue_threshold > 0);
        assert!(cfg.connector_pool.scale_down_idle_ms > 0);
        assert!(cfg.connector_pool.cooldown_ms > 0);
    }

    #[test]
    fn default_config_path_uses_home_config_location() {
        let expected = PathBuf::from("/tmp/example-home")
            .join(".config")
            .join("sharo")
            .join("daemon.toml");
        assert_eq!(
            daemon_config_path_from_home(&PathBuf::from("/tmp/example-home")),
            expected
        );
    }

    #[test]
    fn default_config_path_is_optional_when_home_missing() {
        let _ = default_daemon_config_path();
    }
}
