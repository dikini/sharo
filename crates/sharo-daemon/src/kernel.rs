use std::collections::BTreeMap;
use std::collections::BTreeSet;

use sharo_core::context_resolvers::{ResolverBundle, StaticTextResolver};
use sharo_core::kernel::{KernelApprovalInput, KernelApprovalResult};
use sharo_core::model_connector::{DeterministicConnector, ModelCapabilityFlags, ModelProfile};
use sharo_core::model_connectors::{OllamaConnector, OpenAiCompatibleConnector};
use sharo_core::reasoning::{IdReasoningEngine, ReasoningEnginePort, ReasoningError, ReasoningInput};
use sharo_core::reasoning_context::PolicyConfig;

use crate::config::{
    ConnectorPoolConfig, DaemonConfigFile, ReasoningContextConfig, ReasoningPolicyConfig,
};
use crate::connector_pool::{BlockingPool, PoolError, PoolPolicy};
use crate::store::{Store, SubmitPreparation};

#[derive(Debug, Clone)]
pub enum ConnectorKind {
    Deterministic,
    OpenAiCompatible,
    Ollama,
}

#[derive(Debug, Clone)]
pub struct KernelRuntimeConfig {
    pub connector_kind: ConnectorKind,
    pub profile: ModelProfile,
    pub connector_pool: ConnectorPoolConfig,
    pub reasoning_policy: ReasoningPolicyConfig,
    pub reasoning_context: ReasoningContextConfig,
}

impl KernelRuntimeConfig {
    pub fn from_daemon_config(config: &DaemonConfigFile) -> Result<Self, String> {
        let model_config = &config.model;
        let provider = config
            .model
            .provider
            .as_deref()
            .unwrap_or("deterministic")
            .to_lowercase();
        let profile = ModelProfile {
            profile_id: model_config
                .profile_id
                .clone()
                .unwrap_or_else(|| format!("{provider}-default")),
            provider_id: provider.clone(),
            model_id: model_config
                .model_id
                .clone()
                .unwrap_or_else(|| "mock".to_string()),
            base_url: model_config.base_url.clone(),
            auth_env_key: model_config.auth_env_key.clone(),
            timeout_ms: model_config.timeout_ms.unwrap_or(1_000),
            max_retries: model_config.max_retries.unwrap_or(0),
            capabilities: ModelCapabilityFlags {
                supports_tools: false,
                supports_json_mode: false,
                supports_streaming: false,
                supports_vision: false,
            },
        };

        let connector_kind = match provider.as_str() {
            "deterministic" => ConnectorKind::Deterministic,
            "openai" | "openai_compatible" | "openrouter" | "kimi" | "glm" => {
                ConnectorKind::OpenAiCompatible
            }
            "ollama" => ConnectorKind::Ollama,
            _ => return Err(format!("unsupported_provider provider={provider}")),
        };

        if !matches!(connector_kind, ConnectorKind::Deterministic)
            && profile.base_url.as_deref().unwrap_or("").is_empty()
        {
            return Err(format!("provider_base_url_required provider={provider}"));
        }
        if profile.timeout_ms == 0 {
            return Err("provider_timeout_invalid timeout_ms=0".to_string());
        }
        if config.connector_pool.min_threads == 0 {
            return Err("connector_pool_min_threads_invalid min_threads=0".to_string());
        }
        if config.connector_pool.max_threads < config.connector_pool.min_threads {
            return Err(format!(
                "connector_pool_bounds_invalid min_threads={} max_threads={}",
                config.connector_pool.min_threads, config.connector_pool.max_threads
            ));
        }
        if config.connector_pool.queue_capacity == 0 {
            return Err("connector_pool_queue_capacity_invalid queue_capacity=0".to_string());
        }
        if config.connector_pool.scale_up_queue_threshold == 0 {
            return Err(
                "connector_pool_scale_up_threshold_invalid scale_up_queue_threshold=0".to_string(),
            );
        }
        if config.connector_pool.scale_up_queue_threshold > config.connector_pool.queue_capacity {
            return Err(format!(
                "connector_pool_scale_up_threshold_invalid scale_up_queue_threshold={} queue_capacity={}",
                config.connector_pool.scale_up_queue_threshold, config.connector_pool.queue_capacity
            ));
        }
        if config.connector_pool.scale_down_idle_ms == 0 {
            return Err("connector_pool_idle_invalid scale_down_idle_ms=0".to_string());
        }
        if config.connector_pool.cooldown_ms == 0 {
            return Err("connector_pool_cooldown_invalid cooldown_ms=0".to_string());
        }

        Ok(Self {
            connector_kind,
            profile,
            connector_pool: config.connector_pool.clone(),
            reasoning_policy: config.reasoning_policy.clone(),
            reasoning_context: config.reasoning_context.clone(),
        })
    }
}

#[derive(Clone)]
enum DaemonConnector {
    Deterministic,
    OpenAiCompatible { pool: BlockingPool },
    Ollama { pool: BlockingPool },
}

impl sharo_core::model_connector::ModelConnectorPort for DaemonConnector {
    fn run_turn(
        &self,
        profile: &ModelProfile,
        request: &sharo_core::model_connector::ModelTurnRequest,
    ) -> Result<sharo_core::model_connector::ModelTurnResponse, sharo_core::model_connector::ConnectorError>
    {
        match self {
            DaemonConnector::Deterministic => DeterministicConnector.run_turn(profile, request),
            DaemonConnector::OpenAiCompatible { pool } => execute_via_pool(
                pool,
                OpenAiCompatibleConnector,
                profile,
                request,
            ),
            DaemonConnector::Ollama { pool } => {
                execute_via_pool(pool, OllamaConnector::default(), profile, request)
            }
        }
    }
}

fn execute_via_pool<C: sharo_core::model_connector::ModelConnectorPort + Send + 'static>(
    pool: &BlockingPool,
    connector: C,
    profile: &ModelProfile,
    request: &sharo_core::model_connector::ModelTurnRequest,
) -> Result<sharo_core::model_connector::ModelTurnResponse, sharo_core::model_connector::ConnectorError> {
    let profile = profile.clone();
    let request = request.clone();
    pool
        .execute_with_result(move || connector.run_turn(&profile, &request))
        .map_err(map_pool_error)?
}

fn map_pool_error(error: PoolError) -> sharo_core::model_connector::ConnectorError {
    match error {
        PoolError::Overloaded => {
            sharo_core::model_connector::ConnectorError::Unavailable("connector_pool_overloaded".to_string())
        }
        PoolError::Disconnected => {
            sharo_core::model_connector::ConnectorError::Internal("connector_pool_disconnected".to_string())
        }
        PoolError::WorkerFailed => {
            sharo_core::model_connector::ConnectorError::Internal("connector_pool_worker_failed".to_string())
        }
    }
}

pub struct DaemonKernel {
    reasoning: IdReasoningEngine<DaemonConnector>,
    reasoning_policy: ReasoningPolicyConfig,
}

impl DaemonKernel {
    pub fn new(config: &KernelRuntimeConfig) -> Self {
        let pool = BlockingPool::new(PoolPolicy {
            min_threads: config.connector_pool.min_threads,
            max_threads: config.connector_pool.max_threads,
            queue_capacity: config.connector_pool.queue_capacity,
            scale_up_queue_threshold: config.connector_pool.scale_up_queue_threshold,
            scale_down_idle_ms: config.connector_pool.scale_down_idle_ms,
            cooldown_ms: config.connector_pool.cooldown_ms,
        });
        let connector = match config.connector_kind {
            ConnectorKind::Deterministic => DaemonConnector::Deterministic,
            ConnectorKind::OpenAiCompatible => DaemonConnector::OpenAiCompatible { pool: pool.clone() },
            ConnectorKind::Ollama => DaemonConnector::Ollama { pool: pool.clone() },
        };
        let resolvers = ResolverBundle {
            system: Box::new(StaticTextResolver::new(
                config.reasoning_context.system.as_deref().unwrap_or(""),
                "config-system",
            )),
            persona: Box::new(StaticTextResolver::new(
                config.reasoning_context.persona.as_deref().unwrap_or(""),
                "config-persona",
            )),
            memory: Box::new(StaticTextResolver::new(
                config.reasoning_context.memory.as_deref().unwrap_or(""),
                "config-memory",
            )),
            runtime: Box::new(StaticTextResolver::new(
                config.reasoning_context.runtime.as_deref().unwrap_or(""),
                "config-runtime",
            )),
        };
        Self {
            reasoning: IdReasoningEngine::with_resolvers(connector, config.profile.clone(), resolvers),
            reasoning_policy: config.reasoning_policy.clone(),
        }
    }
}

pub struct DaemonKernelRuntime<'a> {
    store: &'a mut Store,
}

impl<'a> DaemonKernelRuntime<'a> {
    pub fn new(store: &'a mut Store) -> Self {
        Self { store }
    }
}

impl DaemonKernelRuntime<'_> {
    pub fn resolve_approval(
        &mut self,
        input: KernelApprovalInput,
    ) -> Result<KernelApprovalResult, String> {
        let response = self
            .store
            .resolve_approval(&input.approval_id, &input.decision)?;
        Ok(KernelApprovalResult { response })
    }
}

impl DaemonKernel {
    pub fn reason_submit(
        &self,
        preparation: &SubmitPreparation,
        request: &sharo_core::protocol::SubmitTaskOpRequest,
    ) -> Result<sharo_core::reasoning::ReasoningOutcome, ReasoningError> {
        let reasoning_input = ReasoningInput {
            trace_id: format!("trace-{}", preparation.task_id_hint),
            task_id: preparation.task_id_hint.clone(),
            session_id: preparation.session_id_hint.clone(),
            turn_id: preparation.turn_id_hint,
            goal: request.goal.clone(),
            metadata: self.reasoning_metadata(),
        };
        self.reasoning.plan(&reasoning_input)
    }

    pub fn reasoning_metadata(&self) -> BTreeMap<String, String> {
        let mut metadata = BTreeMap::new();
        if let Some(value) = self.reasoning_policy_max_prompt_chars() {
            metadata.insert("policy.max_prompt_chars".to_string(), value.to_string());
        }
        if let Some(value) = self.reasoning_policy_max_memory_lines() {
            metadata.insert("policy.max_memory_lines".to_string(), value.to_string());
        }
        if let Some(value) = self.reasoning_policy_forbidden_runtime_fields() {
            metadata.insert("policy.forbidden_runtime_fields".to_string(), value);
        }
        metadata
    }

    fn reasoning_policy_max_prompt_chars(&self) -> Option<usize> {
        self.reasoning_policy.max_prompt_chars
    }

    fn reasoning_policy_max_memory_lines(&self) -> Option<usize> {
        self.reasoning_policy.max_memory_lines
    }

    fn reasoning_policy_forbidden_runtime_fields(&self) -> Option<String> {
        let mut merged: BTreeSet<String> = PolicyConfig::default()
            .forbidden_runtime_fields
            .into_iter()
            .collect();
        if let Some(configured) = &self.reasoning_policy.forbidden_runtime_fields {
            merged.extend(configured.iter().cloned());
        }
        if merged.is_empty() {
            None
        } else {
            Some(merged.into_iter().collect::<Vec<_>>().join(","))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ConnectorKind, KernelRuntimeConfig};
    use sharo_core::reasoning_context::PolicyConfig;
    use crate::config::{ConnectorPoolConfig, DaemonConfigFile, ModelRuntimeConfig};

    #[test]
    fn openai_like_provider_requires_base_url() {
        let cfg = DaemonConfigFile {
            model: ModelRuntimeConfig {
                provider: Some("openai".to_string()),
                base_url: None,
                ..ModelRuntimeConfig::default()
            },
            connector_pool: ConnectorPoolConfig::default(),
            ..DaemonConfigFile::default()
        };
        let err = KernelRuntimeConfig::from_daemon_config(&cfg).expect_err("expected base_url error");
        assert!(err.contains("provider_base_url_required"));
    }

    #[test]
    fn deterministic_provider_uses_defaults() {
        let cfg = DaemonConfigFile::default();
        let runtime = KernelRuntimeConfig::from_daemon_config(&cfg).expect("runtime config");
        assert!(matches!(runtime.connector_kind, ConnectorKind::Deterministic));
        assert_eq!(runtime.profile.model_id, "mock");
    }

    #[test]
    fn reject_invalid_connector_pool_policy_bounds() {
        let cfg = DaemonConfigFile {
            model: ModelRuntimeConfig::default(),
            connector_pool: ConnectorPoolConfig {
                min_threads: 4,
                max_threads: 2,
                queue_capacity: 64,
                scale_up_queue_threshold: 4,
                scale_down_idle_ms: 1000,
                cooldown_ms: 100,
            },
            ..DaemonConfigFile::default()
        };
        let err = KernelRuntimeConfig::from_daemon_config(&cfg).expect_err("expected bounds error");
        assert!(err.contains("connector_pool_bounds_invalid"));
    }

    #[test]
    fn reject_invalid_connector_pool_threshold() {
        let cfg = DaemonConfigFile {
            model: ModelRuntimeConfig::default(),
            connector_pool: ConnectorPoolConfig {
                min_threads: 1,
                max_threads: 2,
                queue_capacity: 4,
                scale_up_queue_threshold: 8,
                scale_down_idle_ms: 1000,
                cooldown_ms: 100,
            },
            ..DaemonConfigFile::default()
        };
        let err = KernelRuntimeConfig::from_daemon_config(&cfg).expect_err("expected threshold error");
        assert!(err.contains("connector_pool_scale_up_threshold_invalid"));
    }

    #[test]
    fn configured_forbidden_fields_extend_default_redactions() {
        let cfg = DaemonConfigFile {
            reasoning_policy: crate::config::ReasoningPolicyConfig {
                forbidden_runtime_fields: Some(vec!["session_id".to_string(), "token".to_string()]),
                ..crate::config::ReasoningPolicyConfig::default()
            },
            ..DaemonConfigFile::default()
        };
        let runtime = KernelRuntimeConfig::from_daemon_config(&cfg).expect("runtime config");
        let kernel = super::DaemonKernel::new(&runtime);
        let merged = kernel
            .reasoning_policy_forbidden_runtime_fields()
            .expect("merged forbidden fields");
        let fields: std::collections::BTreeSet<&str> = merged.split(',').collect();
        let default_policy = PolicyConfig::default();
        let defaults: std::collections::BTreeSet<&str> = default_policy
            .forbidden_runtime_fields
            .iter()
            .map(String::as_str)
            .collect();
        assert!(defaults.is_subset(&fields));
        assert!(fields.contains("session_id"));
    }
}
