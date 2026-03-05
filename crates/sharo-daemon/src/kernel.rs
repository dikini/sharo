use std::collections::BTreeMap;

use sharo_core::kernel::{
    KernelApprovalInput, KernelApprovalResult, KernelPort, KernelSubmitInput, KernelSubmitResult,
};
use sharo_core::model_connector::{DeterministicConnector, ModelCapabilityFlags, ModelProfile};
use sharo_core::model_connectors::{OllamaConnector, OpenAiCompatibleConnector};
use sharo_core::reasoning::{IdReasoningEngine, ReasoningEnginePort, ReasoningInput};

use crate::config::{ConnectorPoolConfig, DaemonConfigFile};
use crate::connector_pool::{BlockingPool, PoolError, PoolPolicy};
use crate::store::Store;

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
        Self {
            reasoning: IdReasoningEngine::new(connector, config.profile.clone()),
        }
    }
}

pub struct DaemonKernelRuntime<'a> {
    store: &'a mut Store,
    kernel: &'a DaemonKernel,
}

impl<'a> DaemonKernelRuntime<'a> {
    pub fn new(store: &'a mut Store, kernel: &'a DaemonKernel) -> Self {
        Self {
            store,
            kernel,
        }
    }
}

impl KernelPort for DaemonKernelRuntime<'_> {
    fn submit_task(&mut self, input: KernelSubmitInput) -> Result<KernelSubmitResult, String> {
        let task_id_hint = self.store.peek_next_task_id();
        let session_id_hint = input
            .request
            .session_id
            .clone()
            .unwrap_or_else(|| "session-implicit".to_string());
        if let Some(replay) = self
            .store
            .replay_by_idempotency(&session_id_hint, input.request.idempotency_key.as_deref())?
        {
            return Ok(KernelSubmitResult { response: replay });
        }
        let turn_id_hint = self.store.next_turn_id_for_session(&session_id_hint);
        let reasoning = self.kernel.reasoning.plan(&ReasoningInput {
            trace_id: format!("trace-{}", task_id_hint),
            task_id: task_id_hint,
            session_id: session_id_hint.clone(),
            turn_id: turn_id_hint,
            goal: input.request.goal.clone(),
            metadata: BTreeMap::new(),
        })?;

        let response = self.store.submit_task_with_route(
            input.request,
            &session_id_hint,
            &reasoning.route_decision_details,
            &reasoning.model_output_text,
            &reasoning.fit_loop_records,
        )?;
        Ok(KernelSubmitResult { response })
    }

    fn resolve_approval(&mut self, input: KernelApprovalInput) -> Result<KernelApprovalResult, String> {
        let response = self
            .store
            .resolve_approval(&input.approval_id, &input.decision)?;
        Ok(KernelApprovalResult { response })
    }
}

#[cfg(test)]
mod tests {
    use super::{ConnectorKind, KernelRuntimeConfig};
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
        };
        let err = KernelRuntimeConfig::from_daemon_config(&cfg).expect_err("expected threshold error");
        assert!(err.contains("connector_pool_scale_up_threshold_invalid"));
    }
}
