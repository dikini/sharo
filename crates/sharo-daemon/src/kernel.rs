use std::collections::BTreeMap;

use sharo_core::kernel::{
    KernelApprovalInput, KernelApprovalResult, KernelPort, KernelSubmitInput, KernelSubmitResult,
};
use sharo_core::model_connector::{DeterministicConnector, ModelCapabilityFlags, ModelProfile};
use sharo_core::model_connectors::{OllamaConnector, OpenAiCompatibleConnector};
use sharo_core::reasoning::{IdReasoningEngine, ReasoningEnginePort, ReasoningInput};

use crate::config::ModelRuntimeConfig;
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
}

impl KernelRuntimeConfig {
    pub fn from_model_config(config: &ModelRuntimeConfig) -> Result<Self, String> {
        let provider = config
            .provider
            .as_deref()
            .unwrap_or("deterministic")
            .to_lowercase();
        let profile = ModelProfile {
            profile_id: config
                .profile_id
                .clone()
                .unwrap_or_else(|| format!("{provider}-default")),
            provider_id: provider.clone(),
            model_id: config
                .model_id
                .clone()
                .unwrap_or_else(|| "mock".to_string()),
            base_url: config.base_url.clone(),
            auth_env_key: config.auth_env_key.clone(),
            timeout_ms: config.timeout_ms.unwrap_or(1_000),
            max_retries: config.max_retries.unwrap_or(0),
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

        Ok(Self {
            connector_kind,
            profile,
        })
    }
}

#[derive(Debug, Clone, Default)]
enum DaemonConnector {
    #[default]
    Deterministic,
    OpenAiCompatible,
    Ollama,
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
            DaemonConnector::OpenAiCompatible => run_blocking_connector_in_thread(
                OpenAiCompatibleConnector::default(),
                profile,
                request,
            ),
            DaemonConnector::Ollama => {
                run_blocking_connector_in_thread(OllamaConnector::default(), profile, request)
            }
        }
    }
}

fn run_blocking_connector_in_thread<C: sharo_core::model_connector::ModelConnectorPort + Send + 'static>(
    connector: C,
    profile: &ModelProfile,
    request: &sharo_core::model_connector::ModelTurnRequest,
) -> Result<sharo_core::model_connector::ModelTurnResponse, sharo_core::model_connector::ConnectorError> {
    let profile = profile.clone();
    let request = request.clone();
    std::thread::spawn(move || connector.run_turn(&profile, &request))
        .join()
        .map_err(|_| sharo_core::model_connector::ConnectorError::Internal("connector_thread_panicked".to_string()))?
}

pub struct DaemonKernel {
    reasoning: IdReasoningEngine<DaemonConnector>,
}

impl DaemonKernel {
    pub fn new(config: &KernelRuntimeConfig) -> Self {
        let connector = match config.connector_kind {
            ConnectorKind::Deterministic => DaemonConnector::Deterministic,
            ConnectorKind::OpenAiCompatible => DaemonConnector::OpenAiCompatible,
            ConnectorKind::Ollama => DaemonConnector::Ollama,
        };
        Self {
            reasoning: IdReasoningEngine::new(connector, config.profile.clone()),
        }
    }
}

pub struct DaemonKernelRuntime<'a> {
    store: &'a mut Store,
    kernel: DaemonKernel,
}

impl<'a> DaemonKernelRuntime<'a> {
    pub fn new(store: &'a mut Store, config: &KernelRuntimeConfig) -> Self {
        Self {
            store,
            kernel: DaemonKernel::new(config),
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
    use crate::config::ModelRuntimeConfig;

    #[test]
    fn openai_like_provider_requires_base_url() {
        let cfg = ModelRuntimeConfig {
            provider: Some("openai".to_string()),
            base_url: None,
            ..ModelRuntimeConfig::default()
        };
        let err = KernelRuntimeConfig::from_model_config(&cfg).expect_err("expected base_url error");
        assert!(err.contains("provider_base_url_required"));
    }

    #[test]
    fn deterministic_provider_uses_defaults() {
        let cfg = ModelRuntimeConfig::default();
        let runtime = KernelRuntimeConfig::from_model_config(&cfg).expect("runtime config");
        assert!(matches!(runtime.connector_kind, ConnectorKind::Deterministic));
        assert_eq!(runtime.profile.model_id, "mock");
    }
}
