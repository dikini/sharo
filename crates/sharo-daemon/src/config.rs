use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DaemonConfigFile {
    #[serde(default)]
    pub model: ModelRuntimeConfig,
    #[serde(default)]
    pub connector_pool: ConnectorPoolConfig,
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

    let raw = fs::read_to_string(path)
        .map_err(|e| format!("daemon_config_read_failed path={} error={}", path.display(), e))?;
    toml::from_str::<DaemonConfigFile>(&raw)
        .map_err(|e| format!("daemon_config_parse_failed path={} error={}", path.display(), e))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        DaemonConfigFile, daemon_config_path_from_home, default_daemon_config_path,
        load_daemon_config,
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
        assert_eq!(parsed.model.base_url.as_deref(), Some("https://api.openai.com"));
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
