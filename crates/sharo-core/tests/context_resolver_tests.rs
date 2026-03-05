use std::collections::BTreeMap;

use sharo_core::context_resolvers::{
    ComponentResolver, ResolvedContext, ResolverBundle, StaticTextResolver, resolve_context,
};
use sharo_core::model_connector::{
    ModelCapabilityFlags, ModelConnectorPort, ModelProfile, ModelTurnRequest, ModelTurnResponse,
};
use sharo_core::reasoning::{IdReasoningEngine, ReasoningEnginePort, ReasoningInput};

fn test_scope() -> sharo_core::reasoning_context::TurnScope {
    sharo_core::reasoning_context::TurnScope {
        session_id: "session-1".to_string(),
        task_id: "task-1".to_string(),
        turn_id: 1,
        goal: "goal".to_string(),
    }
}

#[test]
fn resolver_contract_is_uniform_for_all_components() {
    let resolver = StaticTextResolver::new("content", "source");
    let scope = test_scope();
    let resolved = resolver.resolve(&scope).expect("resolve");
    assert_eq!(resolved.content, "content");
    assert_eq!(resolved.provenance.source, "source");
}

#[test]
fn component_local_filtering_applies_before_compose() {
    let bundle = ResolverBundle {
        system: Box::new(StaticTextResolver::new("  system  ", "s")),
        persona: Box::new(StaticTextResolver::new(" persona ", "p")),
        memory: Box::new(StaticTextResolver::new(" memory ", "m")),
        runtime: Box::new(StaticTextResolver::new(" runtime ", "r")),
    };
    let scope = test_scope();
    let resolved = resolve_context(&bundle, &scope).expect("resolved context");
    assert_eq!(resolved.system.content, "system");
    assert_eq!(resolved.persona.content, "persona");
    assert_eq!(resolved.memory.content, "memory");
    assert_eq!(resolved.runtime.content, "runtime");
}

#[test]
fn resolver_output_order_is_deterministic() {
    let bundle = ResolverBundle {
        system: Box::new(StaticTextResolver::new("a", "s")),
        persona: Box::new(StaticTextResolver::new("b", "p")),
        memory: Box::new(StaticTextResolver::new("c", "m")),
        runtime: Box::new(StaticTextResolver::new("d", "r")),
    };
    let scope = test_scope();
    let first = resolve_context(&bundle, &scope).expect("first");
    let second = resolve_context(&bundle, &scope).expect("second");
    assert_eq!(first, second);
}

#[derive(Debug, Clone)]
struct EchoConnector;

impl ModelConnectorPort for EchoConnector {
    fn run_turn(
        &self,
        profile: &ModelProfile,
        request: &ModelTurnRequest,
    ) -> Result<ModelTurnResponse, sharo_core::model_connector::ConnectorError> {
        Ok(ModelTurnResponse {
            provider_request_id: Some("req-1".to_string()),
            route_label: format!("{}:{}", profile.provider_id, profile.model_id),
            content: request.prompt.clone(),
        })
    }
}

fn profile() -> ModelProfile {
    ModelProfile {
        profile_id: "id-default".to_string(),
        provider_id: "local".to_string(),
        model_id: "mock".to_string(),
        base_url: None,
        auth_env_key: None,
        timeout_ms: 1000,
        max_retries: 0,
        capabilities: ModelCapabilityFlags {
            supports_tools: false,
            supports_json_mode: false,
            supports_streaming: false,
            supports_vision: false,
        },
    }
}

#[test]
fn kernel_submit_uses_resolved_context_before_model_call() {
    let bundle = ResolverBundle {
        system: Box::new(StaticTextResolver::new("SYSTEM", "s")),
        persona: Box::new(StaticTextResolver::new("PERSONA", "p")),
        memory: Box::new(StaticTextResolver::new("MEMORY", "m")),
        runtime: Box::new(StaticTextResolver::new("RUNTIME", "r")),
    };

    let engine = IdReasoningEngine::with_resolvers(EchoConnector, profile(), bundle);
    let outcome = engine
        .plan(&ReasoningInput {
            trace_id: "trace-1".to_string(),
            task_id: "task-1".to_string(),
            session_id: "session-1".to_string(),
            turn_id: 1,
            goal: "GOAL".to_string(),
            metadata: BTreeMap::new(),
        })
        .expect("reasoning outcome");

    assert!(outcome.model_output_text.contains("GOAL"));
    assert!(outcome.model_output_text.contains("SYSTEM:\nSYSTEM"));
    assert!(outcome.model_output_text.contains("PERSONA:\nPERSONA"));
    assert!(outcome.model_output_text.contains("MEMORY:\nMEMORY"));
    assert!(outcome.model_output_text.contains("RUNTIME:\nRUNTIME"));
    let _typed: ResolvedContext = outcome.resolved_context;
}

#[test]
fn empty_resolved_context_preserves_goal_only_prompt() {
    let engine = IdReasoningEngine::new(EchoConnector, profile());
    let outcome = engine
        .plan(&ReasoningInput {
            trace_id: "trace-1".to_string(),
            task_id: "task-1".to_string(),
            session_id: "session-1".to_string(),
            turn_id: 1,
            goal: "GOAL".to_string(),
            metadata: BTreeMap::new(),
        })
        .expect("reasoning outcome");

    assert_eq!(outcome.model_output_text, "GOAL");
}
