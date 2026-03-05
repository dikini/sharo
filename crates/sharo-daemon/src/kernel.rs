use sharo_core::kernel::{
    KernelApprovalInput, KernelApprovalResult, KernelPort, KernelSubmitInput, KernelSubmitResult,
};
use sharo_core::model_connector::{DeterministicConnector, ModelCapabilityFlags, ModelProfile};
use sharo_core::reasoning::{IdReasoningEngine, ReasoningEnginePort, ReasoningInput};

use crate::store::Store;

pub struct DaemonKernel {
    reasoning: IdReasoningEngine<DeterministicConnector>,
}

impl DaemonKernel {
    pub fn new() -> Self {
        let profile = ModelProfile {
            profile_id: "id-default".to_string(),
            provider_id: "local".to_string(),
            model_id: "mock".to_string(),
            base_url: None,
            auth_env_key: None,
            timeout_ms: 1_000,
            max_retries: 0,
            capabilities: ModelCapabilityFlags {
                supports_tools: false,
                supports_json_mode: false,
                supports_streaming: false,
                supports_vision: false,
            },
        };
        Self {
            reasoning: IdReasoningEngine::new(DeterministicConnector, profile),
        }
    }
}

pub struct DaemonKernelRuntime<'a> {
    store: &'a mut Store,
    kernel: DaemonKernel,
}

impl<'a> DaemonKernelRuntime<'a> {
    pub fn new(store: &'a mut Store) -> Self {
        Self {
            store,
            kernel: DaemonKernel::new(),
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
        let reasoning = self.kernel.reasoning.plan(&ReasoningInput {
            trace_id: format!("trace-{}", task_id_hint),
            task_id: task_id_hint,
            goal: input.request.goal.clone(),
        })?;

        let response = self.store.submit_task_with_route(
            input.request,
            &session_id_hint,
            &reasoning.route_decision_details,
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
