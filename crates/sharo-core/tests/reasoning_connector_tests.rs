use sharo_core::model_connector::{
    DeterministicConnector, ModelCapabilityFlags, ModelConnectorPort, ModelProfile, ModelTurnRequest,
};
use sharo_core::model_connectors::OpenAiCompatibleConnector;
use sharo_core::context_resolvers::{ResolverBundle, StaticTextResolver};
use sharo_core::reasoning::{IdReasoningEngine, ReasoningEnginePort, ReasoningError, ReasoningInput};

fn test_profile() -> ModelProfile {
    ModelProfile {
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
    }
}

#[test]
fn deterministic_connector_returns_provider_route_label() {
    let connector = DeterministicConnector;
    let response = connector
        .run_turn(
            &test_profile(),
            &ModelTurnRequest {
                trace_id: "trace-task-1".to_string(),
                task_id: "task-1".to_string(),
                prompt: "read one context item".to_string(),
                metadata: Default::default(),
            },
        )
        .expect("deterministic connector should succeed");

    assert_eq!(response.route_label, "local_mock");
    assert!(response.content.contains("task=task-1"));
}

#[test]
fn id_reasoning_engine_uses_connector_route_decision() {
    let engine = IdReasoningEngine::new(DeterministicConnector, test_profile());
    let outcome = engine
        .plan(&ReasoningInput {
            trace_id: "trace-task-1".to_string(),
            task_id: "task-1".to_string(),
            session_id: "session-1".to_string(),
            turn_id: 1,
            goal: "read one context item".to_string(),
            metadata: Default::default(),
        })
        .expect("reasoning should succeed");

    assert_eq!(outcome.route_decision_details, "local_mock");
    assert!(outcome.model_output_text.contains("deterministic-response"));
    assert_eq!(outcome.resolved_context.system.provenance.source, "default-system");
    assert!(outcome.fit_loop_records.iter().any(|r| r.decision == "fitted"));
}

#[test]
fn openai_compatible_connector_requires_base_url() {
    let connector = OpenAiCompatibleConnector::default();
    let mut profile = test_profile();
    profile.provider_id = "openai".to_string();
    profile.model_id = "gpt-5-mini".to_string();
    profile.base_url = None;

    let result = connector.run_turn(
        &profile,
        &ModelTurnRequest {
            trace_id: "trace-task-1".to_string(),
            task_id: "task-1".to_string(),
            prompt: "ping".to_string(),
            metadata: Default::default(),
        },
    );

    let error = result.expect_err("missing base_url should fail");
    assert!(matches!(
        error,
        sharo_core::model_connector::ConnectorError::InvalidRequest(_)
    ));
}

#[test]
fn openai_compatible_connector_rejects_zero_timeout_profile() {
    let connector = OpenAiCompatibleConnector::default();
    let mut profile = test_profile();
    profile.provider_id = "openai".to_string();
    profile.model_id = "gpt-5-mini".to_string();
    profile.timeout_ms = 0;

    let result = connector.run_turn(
        &profile,
        &ModelTurnRequest {
            trace_id: "trace-task-1".to_string(),
            task_id: "task-1".to_string(),
            prompt: "ping".to_string(),
            metadata: Default::default(),
        },
    );

    let error = result.expect_err("zero timeout should fail");
    match error {
        sharo_core::model_connector::ConnectorError::InvalidRequest(message) => {
            assert!(message.contains("timeout_ms"));
        }
        other => panic!("unexpected error kind: {other:?}"),
    }
}

#[test]
fn s2_fit_loop_converges_under_budget_pressure() {
    let resolvers = ResolverBundle {
        system: Box::new(StaticTextResolver::new("system=keep-safe", "test-system")),
        persona: Box::new(StaticTextResolver::new("verbosity=high", "test-persona")),
        memory: Box::new(StaticTextResolver::new(
            "m1\nm2\nm3 with many words for compression pressure",
            "test-memory",
        )),
        runtime: Box::new(StaticTextResolver::new("secret=abc123", "test-runtime")),
    };
    let engine = IdReasoningEngine::with_resolvers(DeterministicConnector, test_profile(), resolvers);
    let mut metadata = std::collections::BTreeMap::new();
    metadata.insert("policy.max_prompt_chars".to_string(), "10000".to_string());
    metadata.insert("policy.max_memory_lines".to_string(), "1".to_string());
    metadata.insert(
        "policy.forbidden_runtime_fields".to_string(),
        "secret".to_string(),
    );
    let outcome = engine
        .plan(&ReasoningInput {
            trace_id: "trace-task-s2".to_string(),
            task_id: "task-s2".to_string(),
            session_id: "session-s2".to_string(),
            turn_id: 1,
            goal: "summarize memory and runtime".to_string(),
            metadata,
        })
        .expect("fit loop should converge");

    assert!(outcome.fit_loop_records.iter().any(|r| r.decision == "adjusted"));
    assert_eq!(
        outcome
            .fit_loop_records
            .last()
            .map(|r| r.decision.as_str()),
        Some("fitted")
    );
    assert!(outcome.model_output_text.contains("deterministic-response"));
}

#[test]
fn s4_non_convergent_fit_loop_fails_with_terminal_reason() {
    let engine = IdReasoningEngine::new(DeterministicConnector, test_profile());
    let mut metadata = std::collections::BTreeMap::new();
    metadata.insert("policy.max_prompt_chars".to_string(), "1".to_string());
    let error = engine
        .plan(&ReasoningInput {
            trace_id: "trace-task-s4".to_string(),
            task_id: "task-s4".to_string(),
            session_id: "session-s4".to_string(),
            turn_id: 1,
            goal: "this goal is intentionally too long for the configured budget".to_string(),
            metadata,
        })
        .expect_err("non-convergent fit loop should fail");
    match error {
        ReasoningError::FitLoopFailure { message, records } => {
            assert!(message.contains("context_policy_fit_failed"));
            assert!(!records.is_empty());
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn s3_provider_auth_failure_is_explicit_and_non_success() {
    let mut profile = test_profile();
    profile.provider_id = "openai".to_string();
    profile.model_id = "gpt-5-mini".to_string();
    profile.base_url = Some("https://api.openai.com".to_string());
    profile.auth_env_key = Some("SHARO_TEST_MISSING_OPENAI_KEY".to_string());

    let engine = IdReasoningEngine::new(OpenAiCompatibleConnector::default(), profile);
    let error = engine
        .plan(&ReasoningInput {
            trace_id: "trace-task-s3".to_string(),
            task_id: "task-s3".to_string(),
            session_id: "session-s3".to_string(),
            turn_id: 1,
            goal: "read one context item".to_string(),
            metadata: Default::default(),
        })
        .expect_err("missing auth env var should fail");
    match error {
        ReasoningError::ConnectorFailure { message } => {
            assert!(message.contains("missing auth env var SHARO_TEST_MISSING_OPENAI_KEY"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
