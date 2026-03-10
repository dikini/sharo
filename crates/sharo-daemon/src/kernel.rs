use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use sharo_core::context_resolvers::{
    ComponentProvenance, ComponentResolver, ResolvedComponent, ResolverBundle, StaticTextResolver,
};
use sharo_core::kernel::{KernelApprovalInput, KernelApprovalResult};
use sharo_core::model_connector::{
    ConnectorError, DeterministicConnector, ModelCapabilityFlags, ModelProfile,
    validate_base_url_security,
};
use sharo_core::model_connectors::{OllamaConnector, OpenAiCompatibleConnector};
use sharo_core::protocol::{
    EffectivePolicyBundle, HazelCardPolicyHint, PolicyMergeMode, PolicyRule,
    PrePromptComposeHookInput, RecollectionCardKind, RecollectionPayload, ToolCallRequest,
    ToolCallResponse, validate_pre_prompt_compose_input_value, validate_recollection_payload_value,
};
use sharo_core::reasoning::{
    IdReasoningEngine, ReasoningEnginePort, ReasoningError, ReasoningInput,
};
use sharo_core::reasoning_context::{PolicyConfig, TurnScope};

use crate::config::{
    ConnectorPoolConfig, DaemonConfigFile, HazelCardManifestConfig, HazelManifestConfig,
    HookPolicyDefinitionConfig, PrePromptComposeHookConfig, ReasoningContextConfig,
    ReasoningPolicyConfig,
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
    pub pre_prompt_effective_policy: Option<EffectivePolicyBundle>,
    pub pre_prompt_hook_binding: Option<HookBindingRuntimeConfig>,
    pub hazel_card_policy_hints: Vec<HazelCardPolicyHint>,
}

#[derive(Debug, Clone)]
pub struct HookBindingRuntimeConfig {
    pub id: String,
    pub tool: String,
    pub command: String,
    pub args: Vec<String>,
    pub timeout_ms: u64,
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
        validate_base_url_security(&profile).map_err(|error| match error {
            ConnectorError::InvalidRequest(message) => message,
            other => format!("provider_base_url_validation_failed error={other:?}"),
        })?;
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
                config.connector_pool.scale_up_queue_threshold,
                config.connector_pool.queue_capacity
            ));
        }
        if config.connector_pool.scale_down_idle_ms == 0 {
            return Err("connector_pool_idle_invalid scale_down_idle_ms=0".to_string());
        }
        if config.connector_pool.cooldown_ms == 0 {
            return Err("connector_pool_cooldown_invalid cooldown_ms=0".to_string());
        }
        let pre_prompt_hook_binding =
            validate_pre_prompt_compose_hook(&config.reasoning_hooks.pre_prompt_compose)?;
        let hazel_card_policy_hints = build_hazel_card_policy_hints(
            &config.hazel_manifest,
            config
                .reasoning_hooks
                .pre_prompt_compose
                .strict_unknown_policy_ids,
            &config.hook_policies,
        )?;
        let pre_prompt_effective_policy = build_effective_policy_bundle(
            &config.reasoning_hooks.pre_prompt_compose,
            &hazel_card_policy_hints,
            &config.hook_policies,
        )?;

        Ok(Self {
            connector_kind,
            profile,
            connector_pool: config.connector_pool.clone(),
            reasoning_policy: config.reasoning_policy.clone(),
            reasoning_context: config.reasoning_context.clone(),
            pre_prompt_effective_policy,
            pre_prompt_hook_binding,
            hazel_card_policy_hints,
        })
    }
}

fn validate_pre_prompt_compose_hook(
    config: &PrePromptComposeHookConfig,
) -> Result<Option<HookBindingRuntimeConfig>, String> {
    let composition = config.composition.as_deref().unwrap_or("single");
    if composition != "single" {
        return Err(format!(
            "pre_prompt_compose_composition_invalid composition={} expected=single",
            composition
        ));
    }
    let binding_count = config.bindings.as_ref().map_or(0, Vec::len);
    if binding_count > 1 {
        return Err(format!(
            "pre_prompt_compose_bindings_invalid count={} max=1",
            binding_count
        ));
    }
    let Some(binding) = config
        .bindings
        .as_ref()
        .and_then(|bindings| bindings.first())
    else {
        return Ok(None);
    };
    let Some(command) = binding.command.clone() else {
        return Err(format!(
            "pre_prompt_compose_binding_command_missing id={}",
            binding.id
        ));
    };
    Ok(Some(HookBindingRuntimeConfig {
        id: binding.id.clone(),
        tool: binding.tool.clone(),
        command,
        args: binding.args.clone().unwrap_or_default(),
        timeout_ms: binding.timeout_ms.unwrap_or(2_000),
    }))
}

fn parse_policy_rule(raw: &str) -> Result<PolicyRule, String> {
    match raw {
        "label_guesses" => Ok(PolicyRule::LabelGuesses),
        "prefer_supported_facts" => Ok(PolicyRule::PreferSupportedFacts),
        _ => Err(format!("hook_policy_rule_invalid rule={}", raw)),
    }
}

fn build_effective_policy_bundle(
    hook: &PrePromptComposeHookConfig,
    card_policy_hints: &[HazelCardPolicyHint],
    registry: &BTreeMap<String, HookPolicyDefinitionConfig>,
) -> Result<Option<EffectivePolicyBundle>, String> {
    let mut policy_ids = hook.default_policy_ids.clone().unwrap_or_default();
    for hint in card_policy_hints {
        for policy_id in &hint.policy_ids {
            policy_ids.push(policy_id.clone());
        }
    }
    if policy_ids.is_empty() {
        return Ok(None);
    }
    policy_ids.sort();
    policy_ids.dedup();

    let strict_unknown = hook.strict_unknown_policy_ids.unwrap_or(true);
    let mut rules = Vec::new();
    for policy_id in &policy_ids {
        let Some(definition) = registry.get(policy_id) else {
            if strict_unknown {
                return Err(format!(
                    "pre_prompt_compose_policy_missing policy_id={}",
                    policy_id
                ));
            }
            continue;
        };
        for raw_rule in &definition.rules {
            rules.push(parse_policy_rule(raw_rule)?);
        }
    }
    rules.sort_by_key(|rule| match rule {
        PolicyRule::LabelGuesses => 0,
        PolicyRule::PreferSupportedFacts => 1,
    });
    rules.dedup();

    Ok(Some(EffectivePolicyBundle::new(
        policy_ids,
        PolicyMergeMode::StrictestWins,
        rules,
    )))
}

fn parse_card_kind(raw: &str) -> Result<RecollectionCardKind, String> {
    match raw {
        "soft_recollection" => Ok(RecollectionCardKind::SoftRecollection),
        "strong_constraint" => Ok(RecollectionCardKind::StrongConstraint),
        "supporting_context" => Ok(RecollectionCardKind::SupportingContext),
        "association_cue" => Ok(RecollectionCardKind::AssociationCue),
        "do_not_use" => Ok(RecollectionCardKind::DoNotUse),
        _ => Err(format!("hazel_manifest_card_kind_invalid kind={raw}")),
    }
}

fn kind_order(kind: &RecollectionCardKind) -> usize {
    match kind {
        RecollectionCardKind::SoftRecollection => 0,
        RecollectionCardKind::StrongConstraint => 1,
        RecollectionCardKind::SupportingContext => 2,
        RecollectionCardKind::AssociationCue => 3,
        RecollectionCardKind::DoNotUse => 4,
    }
}

fn build_hazel_card_policy_hints(
    manifest: &HazelManifestConfig,
    strict_unknown_policy_ids: Option<bool>,
    registry: &BTreeMap<String, HookPolicyDefinitionConfig>,
) -> Result<Vec<HazelCardPolicyHint>, String> {
    let strict_unknown = strict_unknown_policy_ids.unwrap_or(true);
    let mut hints = Vec::new();
    for HazelCardManifestConfig {
        kind,
        policy_ids,
        max_cards,
    } in &manifest.cards
    {
        let kind = parse_card_kind(kind)?;
        let mut policy_ids = policy_ids.clone();
        policy_ids.sort();
        policy_ids.dedup();
        if strict_unknown {
            for policy_id in &policy_ids {
                if !registry.contains_key(policy_id) {
                    return Err(format!(
                        "pre_prompt_compose_policy_missing policy_id={}",
                        policy_id
                    ));
                }
            }
        }
        hints.push(HazelCardPolicyHint {
            kind,
            policy_ids,
            max_cards: *max_cards,
        });
    }
    hints.sort_by(|left, right| {
        kind_order(&left.kind)
            .cmp(&kind_order(&right.kind))
            .then_with(|| left.policy_ids.cmp(&right.policy_ids))
    });
    Ok(hints)
}

fn render_policy_rule(rule: &PolicyRule) -> &'static str {
    match rule {
        PolicyRule::LabelGuesses => "label guesses explicitly",
        PolicyRule::PreferSupportedFacts => "prefer supported facts for assertions",
    }
}

fn render_card_kind(kind: &RecollectionCardKind) -> &'static str {
    match kind {
        RecollectionCardKind::SoftRecollection => "soft_recollection",
        RecollectionCardKind::StrongConstraint => "strong_constraint",
        RecollectionCardKind::SupportingContext => "supporting_context",
        RecollectionCardKind::AssociationCue => "association_cue",
        RecollectionCardKind::DoNotUse => "do_not_use",
    }
}

fn compose_pre_prompt_memory_block(
    base_memory: Option<&str>,
    bundle: Option<&EffectivePolicyBundle>,
    card_policy_hints: &[HazelCardPolicyHint],
    recollection: Option<&RecollectionPayload>,
) -> String {
    let mut sections = Vec::new();
    let base = base_memory.unwrap_or("").trim();
    if !base.is_empty() {
        sections.push(base.to_string());
    }

    if let Some(bundle) = bundle {
        let mut lines = Vec::new();
        lines.push("HAZEL_POLICY_CONTROL:".to_string());
        lines.push(format!(
            "POLICY_IDS: {}",
            bundle.effective_policy_ids.join(",")
        ));
        for rule in &bundle.rules {
            lines.push(format!("RULE: {}", render_policy_rule(rule)));
        }
        sections.push(lines.join("\n"));
    }

    if !card_policy_hints.is_empty() {
        let mut lines = Vec::new();
        lines.push("HAZEL_CARD_POLICY:".to_string());
        for hint in card_policy_hints {
            lines.push(format!(
                "CARD_KIND: {} POLICY_IDS: {} MAX_CARDS: {}",
                render_card_kind(&hint.kind),
                hint.policy_ids.join(","),
                hint.max_cards.unwrap_or(0)
            ));
        }
        sections.push(lines.join("\n"));
    }

    if let Some(recollection) = recollection {
        let mut lines = Vec::new();
        lines.push("HAZEL_RECOLLECTIONS:".to_string());
        lines.push(format!("POLICY_IDS: {}", recollection.policy_ids.join(",")));
        for card in &recollection.cards {
            lines.push(format!(
                "CARD {} [{}|{}]: {} => {}",
                card.card_id,
                render_card_kind(&card.kind),
                match card.state {
                    sharo_core::protocol::RecollectionCardState::Candidate => "candidate",
                    sharo_core::protocol::RecollectionCardState::Active => "active",
                    sharo_core::protocol::RecollectionCardState::Contested => "contested",
                    sharo_core::protocol::RecollectionCardState::Deprecated => "deprecated",
                },
                card.subject,
                card.text
            ));
            for provenance in &card.provenance {
                lines.push(format!("PROVENANCE: {}", provenance.source_ref));
            }
        }
        sections.push(lines.join("\n"));
    }

    sections.join("\n\n")
}

#[derive(Clone)]
struct PrePromptComposeMemoryResolver {
    base_memory: Option<String>,
    runtime_context: Option<String>,
    effective_policy: Option<EffectivePolicyBundle>,
    card_policy_hints: Vec<HazelCardPolicyHint>,
    hook_binding: Option<HookBindingRuntimeConfig>,
}

impl PrePromptComposeMemoryResolver {
    fn run_pre_prompt_hook(
        &self,
        scope: &TurnScope,
    ) -> Result<Option<RecollectionPayload>, String> {
        let Some(binding) = &self.hook_binding else {
            return Ok(None);
        };
        let input = PrePromptComposeHookInput {
            session_id: scope.session_id.clone(),
            task_id: scope.task_id.clone(),
            goal: scope.goal.clone(),
            runtime: self
                .runtime_context
                .clone()
                .unwrap_or_else(|| "daemon".to_string()),
            policy_ids: self
                .effective_policy
                .as_ref()
                .map(|bundle| bundle.effective_policy_ids.clone())
                .unwrap_or_default(),
            card_policy_hints: self.card_policy_hints.clone(),
        };
        let input_value = serde_json::to_value(input)
            .map_err(|error| format!("pre_prompt_input_encode_failed error={error}"))?;
        let input_value =
            validate_pre_prompt_compose_input_value(&input_value).and_then(|validated| {
                serde_json::to_value(validated)
                    .map_err(|error| format!("pre_prompt_input_encode_failed error={error}"))
            })?;
        let request = ToolCallRequest {
            tool: binding.tool.clone(),
            input: input_value,
        };
        let response = run_stdio_tool_call(binding, &request)?;
        if !response.ok {
            let error = response
                .error
                .unwrap_or_else(|| "pre_prompt_tool_error_unknown".to_string());
            return Err(format!(
                "pre_prompt_tool_failed binding={} error={error}",
                binding.id
            ));
        }
        let Some(output) = response.output else {
            return Err(format!(
                "pre_prompt_tool_output_missing binding={}",
                binding.id
            ));
        };
        let recollection = validate_recollection_payload_value(&output)?;
        Ok(Some(recollection))
    }
}

impl ComponentResolver for PrePromptComposeMemoryResolver {
    fn resolve(&self, scope: &TurnScope) -> Result<ResolvedComponent, String> {
        let recollection = self.run_pre_prompt_hook(scope)?;
        let content = compose_pre_prompt_memory_block(
            self.base_memory.as_deref(),
            self.effective_policy.as_ref(),
            &self.card_policy_hints,
            recollection.as_ref(),
        );
        Ok(ResolvedComponent {
            content,
            provenance: ComponentProvenance {
                source: if self.hook_binding.is_some() {
                    "config-memory+pre-prompt-hook"
                } else {
                    "config-memory"
                }
                .to_string(),
                applied_filters: vec![],
            },
        })
    }
}

fn run_stdio_tool_call(
    binding: &HookBindingRuntimeConfig,
    request: &ToolCallRequest,
) -> Result<ToolCallResponse, String> {
    let mut child = Command::new(&binding.command)
        .args(&binding.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            format!(
                "pre_prompt_tool_spawn_failed binding={} command={} error={error}",
                binding.id, binding.command
            )
        })?;

    {
        let Some(mut stdin) = child.stdin.take() else {
            return Err(format!(
                "pre_prompt_tool_stdin_unavailable binding={}",
                binding.id
            ));
        };
        let payload = serde_json::to_string(request)
            .map_err(|error| format!("pre_prompt_tool_request_encode_failed error={error}"))?;
        writeln!(stdin, "{payload}")
            .map_err(|error| format!("pre_prompt_tool_request_write_failed error={error}"))?;
    }

    let Some(stdout) = child.stdout.take() else {
        return Err(format!(
            "pre_prompt_tool_stdout_unavailable binding={}",
            binding.id
        ));
    };
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        let read_result = reader.read_line(&mut line);
        let _ = sender.send((read_result, line));
    });

    let timeout = Duration::from_millis(binding.timeout_ms);
    let (read_result, line) = match receiver.recv_timeout(timeout) {
        Ok(value) => value,
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "pre_prompt_tool_timeout binding={} timeout_ms={}",
                binding.id, binding.timeout_ms
            ));
        }
    };
    read_result.map_err(|error| format!("pre_prompt_tool_response_read_failed error={error}"))?;

    let status = child
        .wait()
        .map_err(|error| format!("pre_prompt_tool_wait_failed error={error}"))?;
    let mut stderr = String::new();
    if let Some(mut stderr_pipe) = child.stderr.take() {
        let _ = stderr_pipe.read_to_string(&mut stderr);
    }
    if !status.success() {
        return Err(format!(
            "pre_prompt_tool_nonzero_exit binding={} status={} stderr={}",
            binding.id,
            status,
            stderr.trim()
        ));
    }
    if line.trim().is_empty() {
        return Err(format!(
            "pre_prompt_tool_response_empty binding={}",
            binding.id
        ));
    }
    serde_json::from_str::<ToolCallResponse>(line.trim()).map_err(|error| {
        format!(
            "pre_prompt_tool_response_parse_failed binding={} error={error}",
            binding.id
        )
    })
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
    ) -> Result<
        sharo_core::model_connector::ModelTurnResponse,
        sharo_core::model_connector::ConnectorError,
    > {
        match self {
            DaemonConnector::Deterministic => DeterministicConnector.run_turn(profile, request),
            DaemonConnector::OpenAiCompatible { pool } => {
                execute_via_pool(pool, OpenAiCompatibleConnector, profile, request)
            }
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
) -> Result<
    sharo_core::model_connector::ModelTurnResponse,
    sharo_core::model_connector::ConnectorError,
> {
    let profile = profile.clone();
    let request = request.clone();
    pool.execute_with_result(move || connector.run_turn(&profile, &request))
        .map_err(map_pool_error)?
}

fn map_pool_error(error: PoolError) -> sharo_core::model_connector::ConnectorError {
    match error {
        PoolError::Overloaded => sharo_core::model_connector::ConnectorError::Unavailable(
            "connector_pool_overloaded".to_string(),
        ),
        PoolError::Disconnected => sharo_core::model_connector::ConnectorError::Internal(
            "connector_pool_disconnected".to_string(),
        ),
        PoolError::WorkerFailed => sharo_core::model_connector::ConnectorError::Internal(
            "connector_pool_worker_failed".to_string(),
        ),
    }
}

pub struct DaemonKernel {
    reasoning: IdReasoningEngine<DaemonConnector>,
    reasoning_policy: ReasoningPolicyConfig,
    pre_prompt_effective_policy: Option<EffectivePolicyBundle>,
    hazel_card_policy_hints: Vec<HazelCardPolicyHint>,
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
            ConnectorKind::OpenAiCompatible => {
                DaemonConnector::OpenAiCompatible { pool: pool.clone() }
            }
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
            memory: Box::new(PrePromptComposeMemoryResolver {
                base_memory: config.reasoning_context.memory.clone(),
                runtime_context: config.reasoning_context.runtime.clone(),
                effective_policy: config.pre_prompt_effective_policy.clone(),
                card_policy_hints: config.hazel_card_policy_hints.clone(),
                hook_binding: config.pre_prompt_hook_binding.clone(),
            }),
            runtime: Box::new(StaticTextResolver::new(
                config.reasoning_context.runtime.as_deref().unwrap_or(""),
                "config-runtime",
            )),
        };
        Self {
            reasoning: IdReasoningEngine::with_resolvers(
                connector,
                config.profile.clone(),
                resolvers,
            ),
            reasoning_policy: config.reasoning_policy.clone(),
            pre_prompt_effective_policy: config.pre_prompt_effective_policy.clone(),
            hazel_card_policy_hints: config.hazel_card_policy_hints.clone(),
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
        if let Some(bundle) = &self.pre_prompt_effective_policy {
            metadata.insert(
                "policy.effective_ids".to_string(),
                bundle.effective_policy_ids.join(","),
            );
            let rule_names = bundle
                .rules
                .iter()
                .map(|rule| match rule {
                    PolicyRule::LabelGuesses => "label_guesses",
                    PolicyRule::PreferSupportedFacts => "prefer_supported_facts",
                })
                .collect::<Vec<_>>();
            metadata.insert("policy.effective_rules".to_string(), rule_names.join(","));
        }
        if !self.hazel_card_policy_hints.is_empty() {
            let serialized = self
                .hazel_card_policy_hints
                .iter()
                .map(|hint| {
                    format!(
                        "{}:{}:{}",
                        render_card_kind(&hint.kind),
                        hint.max_cards.unwrap_or(0),
                        hint.policy_ids.join("+")
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            metadata.insert("policy.card_hints".to_string(), serialized);
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
    use std::collections::BTreeMap;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{ConnectorKind, KernelRuntimeConfig};
    use crate::config::{
        ConnectorPoolConfig, DaemonConfigFile, HookBindingConfig, HookPolicyDefinitionConfig,
        ModelRuntimeConfig, PrePromptComposeHookConfig, ReasoningHooksConfig,
    };
    use crate::store::SubmitPreparation;
    use sharo_core::protocol::{EffectivePolicyBundle, PolicyMergeMode, PolicyRule};
    use sharo_core::reasoning_context::PolicyConfig;

    fn write_mock_mcp_script(response_line: &str) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("sharo-hazel-mcp-mock-{nanos}.sh"));
        let content = format!(
            "#!/usr/bin/env bash\nread _line\necho '{}'\n",
            response_line.replace('\'', "'\"'\"'")
        );
        fs::write(&path, content).expect("write mock mcp script");
        let mut permissions = fs::metadata(&path).expect("metadata").permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&path, permissions).expect("chmod script");
        path.to_string_lossy().to_string()
    }

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
        let err =
            KernelRuntimeConfig::from_daemon_config(&cfg).expect_err("expected base_url error");
        assert!(err.contains("provider_base_url_required"));
    }

    #[test]
    fn authenticated_provider_rejects_cleartext_remote_base_url() {
        let cfg = DaemonConfigFile {
            model: ModelRuntimeConfig {
                provider: Some("openai".to_string()),
                base_url: Some("http://example.com".to_string()),
                auth_env_key: Some("SHARO_TEST_OPENAI_KEY".to_string()),
                ..ModelRuntimeConfig::default()
            },
            connector_pool: ConnectorPoolConfig::default(),
            ..DaemonConfigFile::default()
        };
        let err = KernelRuntimeConfig::from_daemon_config(&cfg)
            .expect_err("authenticated cleartext remote base_url should fail");
        assert!(err.contains("provider_base_url_insecure"));
    }

    #[test]
    fn authenticated_provider_allows_loopback_http_base_url() {
        let cfg = DaemonConfigFile {
            model: ModelRuntimeConfig {
                provider: Some("openai".to_string()),
                base_url: Some("http://127.0.0.1:8080".to_string()),
                auth_env_key: Some("SHARO_TEST_OPENAI_KEY".to_string()),
                ..ModelRuntimeConfig::default()
            },
            connector_pool: ConnectorPoolConfig::default(),
            ..DaemonConfigFile::default()
        };
        let runtime = KernelRuntimeConfig::from_daemon_config(&cfg)
            .expect("loopback http base_url should remain allowed");
        assert_eq!(
            runtime.profile.base_url.as_deref(),
            Some("http://127.0.0.1:8080")
        );
    }

    #[test]
    fn authenticated_provider_allows_loopback_ip_literal_http_base_url() {
        let cfg = DaemonConfigFile {
            model: ModelRuntimeConfig {
                provider: Some("openai".to_string()),
                base_url: Some("http://127.0.0.2:8080".to_string()),
                auth_env_key: Some("SHARO_TEST_OPENAI_KEY".to_string()),
                ..ModelRuntimeConfig::default()
            },
            connector_pool: ConnectorPoolConfig::default(),
            ..DaemonConfigFile::default()
        };
        let runtime = KernelRuntimeConfig::from_daemon_config(&cfg)
            .expect("loopback IP literal http base_url should remain allowed");
        assert_eq!(
            runtime.profile.base_url.as_deref(),
            Some("http://127.0.0.2:8080")
        );
    }

    #[test]
    fn deterministic_provider_uses_defaults() {
        let cfg = DaemonConfigFile::default();
        let runtime = KernelRuntimeConfig::from_daemon_config(&cfg).expect("runtime config");
        assert!(matches!(
            runtime.connector_kind,
            ConnectorKind::Deterministic
        ));
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
        let err =
            KernelRuntimeConfig::from_daemon_config(&cfg).expect_err("expected threshold error");
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

    #[test]
    fn pre_prompt_compose_rejects_multiple_bindings() {
        let cfg = DaemonConfigFile {
            reasoning_hooks: ReasoningHooksConfig {
                pre_prompt_compose: PrePromptComposeHookConfig {
                    composition: Some("single".to_string()),
                    bindings: Some(vec![
                        HookBindingConfig {
                            id: "hazel-a".to_string(),
                            tool: "hazel.recollect".to_string(),
                            command: Some("hazel-mcp".to_string()),
                            args: None,
                            timeout_ms: None,
                        },
                        HookBindingConfig {
                            id: "hazel-b".to_string(),
                            tool: "hazel.recollect".to_string(),
                            command: Some("hazel-mcp".to_string()),
                            args: None,
                            timeout_ms: None,
                        },
                    ]),
                    default_policy_ids: None,
                    strict_unknown_policy_ids: Some(true),
                },
            },
            ..DaemonConfigFile::default()
        };
        let err = KernelRuntimeConfig::from_daemon_config(&cfg)
            .expect_err("multiple bindings should fail under single composition");
        assert!(err.contains("pre_prompt_compose_bindings_invalid"));
    }

    #[test]
    fn pre_prompt_compose_rejects_unknown_policy_id_when_strict() {
        let cfg = DaemonConfigFile {
            reasoning_hooks: ReasoningHooksConfig {
                pre_prompt_compose: PrePromptComposeHookConfig {
                    composition: Some("single".to_string()),
                    bindings: None,
                    default_policy_ids: Some(vec!["unknown.v1".to_string()]),
                    strict_unknown_policy_ids: Some(true),
                },
            },
            ..DaemonConfigFile::default()
        };
        let err = KernelRuntimeConfig::from_daemon_config(&cfg)
            .expect_err("unknown strict policy id should fail");
        assert!(err.contains("pre_prompt_compose_policy_missing"));
    }

    #[test]
    fn pre_prompt_compose_policy_ids_emit_effective_metadata() {
        let mut hook_policies = BTreeMap::new();
        hook_policies.insert(
            "hunch.v1".to_string(),
            HookPolicyDefinitionConfig {
                rules: vec!["label_guesses".to_string()],
            },
        );
        hook_policies.insert(
            "safety.strict.v1".to_string(),
            HookPolicyDefinitionConfig {
                rules: vec!["prefer_supported_facts".to_string()],
            },
        );
        let cfg = DaemonConfigFile {
            reasoning_hooks: ReasoningHooksConfig {
                pre_prompt_compose: PrePromptComposeHookConfig {
                    composition: Some("single".to_string()),
                    bindings: None,
                    default_policy_ids: Some(vec![
                        "safety.strict.v1".to_string(),
                        "hunch.v1".to_string(),
                        "hunch.v1".to_string(),
                    ]),
                    strict_unknown_policy_ids: Some(true),
                },
            },
            hook_policies,
            ..DaemonConfigFile::default()
        };

        let runtime = KernelRuntimeConfig::from_daemon_config(&cfg).expect("runtime config");
        let kernel = super::DaemonKernel::new(&runtime);
        let metadata = kernel.reasoning_metadata();
        assert_eq!(
            metadata.get("policy.effective_ids").map(String::as_str),
            Some("hunch.v1,safety.strict.v1")
        );
        assert_eq!(
            metadata.get("policy.effective_rules").map(String::as_str),
            Some("label_guesses,prefer_supported_facts")
        );
    }

    #[test]
    fn pre_prompt_compose_manifest_policy_ids_are_additive_and_deterministic() {
        let mut hook_policies = BTreeMap::new();
        hook_policies.insert(
            "hunch.v1".to_string(),
            HookPolicyDefinitionConfig {
                rules: vec!["label_guesses".to_string()],
            },
        );
        hook_policies.insert(
            "safety.strict.v1".to_string(),
            HookPolicyDefinitionConfig {
                rules: vec!["prefer_supported_facts".to_string()],
            },
        );
        let cfg = DaemonConfigFile {
            reasoning_hooks: ReasoningHooksConfig {
                pre_prompt_compose: PrePromptComposeHookConfig {
                    composition: Some("single".to_string()),
                    bindings: None,
                    default_policy_ids: Some(vec!["safety.strict.v1".to_string()]),
                    strict_unknown_policy_ids: Some(true),
                },
            },
            hazel_manifest: crate::config::HazelManifestConfig {
                cards: vec![crate::config::HazelCardManifestConfig {
                    kind: "association_cue".to_string(),
                    policy_ids: vec!["hunch.v1".to_string(), "hunch.v1".to_string()],
                    max_cards: Some(3),
                }],
            },
            hook_policies,
            ..DaemonConfigFile::default()
        };

        let runtime = KernelRuntimeConfig::from_daemon_config(&cfg).expect("runtime config");
        let kernel = super::DaemonKernel::new(&runtime);
        let metadata = kernel.reasoning_metadata();
        assert_eq!(
            metadata.get("policy.effective_ids").map(String::as_str),
            Some("hunch.v1,safety.strict.v1")
        );
        assert_eq!(
            metadata.get("policy.card_hints").map(String::as_str),
            Some("association_cue:3:hunch.v1")
        );
    }

    #[test]
    fn pre_prompt_compose_injects_policy_instruction_memory_block() {
        let mut hook_policies = BTreeMap::new();
        hook_policies.insert(
            "hunch.v1".to_string(),
            HookPolicyDefinitionConfig {
                rules: vec!["label_guesses".to_string()],
            },
        );
        let cfg = DaemonConfigFile {
            reasoning_context: crate::config::ReasoningContextConfig {
                memory: Some("existing memory".to_string()),
                ..crate::config::ReasoningContextConfig::default()
            },
            reasoning_hooks: ReasoningHooksConfig {
                pre_prompt_compose: PrePromptComposeHookConfig {
                    composition: Some("single".to_string()),
                    bindings: None,
                    default_policy_ids: Some(vec!["hunch.v1".to_string()]),
                    strict_unknown_policy_ids: Some(true),
                },
            },
            hook_policies,
            ..DaemonConfigFile::default()
        };
        let runtime = KernelRuntimeConfig::from_daemon_config(&cfg).expect("runtime config");
        let kernel = super::DaemonKernel::new(&runtime);
        let outcome = kernel
            .reason_submit(
                &SubmitPreparation {
                    task_id_hint: "task-000001".to_string(),
                    task_id_sequence_hint: 1,
                    session_id_hint: "session-000001".to_string(),
                    turn_id_hint: 1,
                },
                &sharo_core::protocol::SubmitTaskOpRequest {
                    session_id: Some("session-000001".to_string()),
                    goal: "explain memory".to_string(),
                    idempotency_key: None,
                },
            )
            .expect("reasoning outcome");
        assert!(outcome.model_output_text.contains("existing memory"));
        assert!(outcome.model_output_text.contains("POLICY_IDS: hunch.v1"));
        assert!(
            outcome
                .model_output_text
                .contains("RULE: label guesses explicitly")
        );
    }

    #[test]
    fn pre_prompt_compose_single_binding_injects_canonical_recollection_payload() {
        let script_path = write_mock_mcp_script(
            r#"{"ok":true,"output":{"policy_ids":["hunch.v1"],"cards":[{"card_id":"card-1","kind":"association_cue","state":"candidate","subject":"memory-system","text":"Use prior architecture context","provenance":[{"source_ref":"note:hazel"}],"policy_ids":["hunch.v1"]}]}}"#,
        );
        let mut hook_policies = BTreeMap::new();
        hook_policies.insert(
            "hunch.v1".to_string(),
            HookPolicyDefinitionConfig {
                rules: vec!["label_guesses".to_string()],
            },
        );
        let cfg = DaemonConfigFile {
            reasoning_hooks: ReasoningHooksConfig {
                pre_prompt_compose: PrePromptComposeHookConfig {
                    composition: Some("single".to_string()),
                    bindings: Some(vec![HookBindingConfig {
                        id: "hazel".to_string(),
                        tool: "hazel.recollect".to_string(),
                        command: Some(script_path),
                        args: Some(vec![]),
                        timeout_ms: Some(500),
                    }]),
                    default_policy_ids: Some(vec!["hunch.v1".to_string()]),
                    strict_unknown_policy_ids: Some(true),
                },
            },
            hook_policies,
            ..DaemonConfigFile::default()
        };
        let runtime = KernelRuntimeConfig::from_daemon_config(&cfg).expect("runtime config");
        let kernel = super::DaemonKernel::new(&runtime);
        let outcome = kernel
            .reason_submit(
                &SubmitPreparation {
                    task_id_hint: "task-000001".to_string(),
                    task_id_sequence_hint: 1,
                    session_id_hint: "session-000001".to_string(),
                    turn_id_hint: 1,
                },
                &sharo_core::protocol::SubmitTaskOpRequest {
                    session_id: Some("session-000001".to_string()),
                    goal: "explain memory".to_string(),
                    idempotency_key: None,
                },
            )
            .expect("reasoning outcome");
        assert!(outcome.model_output_text.contains("HAZEL_RECOLLECTIONS:"));
        assert!(
            outcome
                .model_output_text
                .contains("Use prior architecture context")
        );
    }

    #[test]
    fn pre_prompt_compose_rejects_schema_mismatch_output() {
        let script_path = write_mock_mcp_script(
            r#"{"ok":true,"output":{"policy_ids":["hunch.v1"],"cards":[],"unexpected":true}}"#,
        );
        let cfg = DaemonConfigFile {
            reasoning_hooks: ReasoningHooksConfig {
                pre_prompt_compose: PrePromptComposeHookConfig {
                    composition: Some("single".to_string()),
                    bindings: Some(vec![HookBindingConfig {
                        id: "hazel".to_string(),
                        tool: "hazel.recollect".to_string(),
                        command: Some(script_path),
                        args: Some(vec![]),
                        timeout_ms: Some(500),
                    }]),
                    default_policy_ids: None,
                    strict_unknown_policy_ids: Some(true),
                },
            },
            ..DaemonConfigFile::default()
        };
        let runtime = KernelRuntimeConfig::from_daemon_config(&cfg).expect("runtime config");
        let kernel = super::DaemonKernel::new(&runtime);
        let error = kernel
            .reason_submit(
                &SubmitPreparation {
                    task_id_hint: "task-000001".to_string(),
                    task_id_sequence_hint: 1,
                    session_id_hint: "session-000001".to_string(),
                    turn_id_hint: 1,
                },
                &sharo_core::protocol::SubmitTaskOpRequest {
                    session_id: Some("session-000001".to_string()),
                    goal: "explain memory".to_string(),
                    idempotency_key: None,
                },
            )
            .expect_err("schema mismatch should fail");
        match error {
            sharo_core::reasoning::ReasoningError::ResolveFailure { message } => {
                assert!(message.contains("pre_prompt_output_schema_invalid"));
            }
            other => panic!("unexpected error kind: {other:?}"),
        }
    }

    #[test]
    fn pre_prompt_compose_rejects_semantic_lint_output() {
        let script_path = write_mock_mcp_script(
            r#"{"ok":true,"output":{"policy_ids":["hunch.v1"],"cards":[{"card_id":"card-1","kind":"association_cue","state":"candidate","subject":"x","text":"x","provenance":[],"policy_ids":["hunch.v1"]}]}}"#,
        );
        let cfg = DaemonConfigFile {
            reasoning_hooks: ReasoningHooksConfig {
                pre_prompt_compose: PrePromptComposeHookConfig {
                    composition: Some("single".to_string()),
                    bindings: Some(vec![HookBindingConfig {
                        id: "hazel".to_string(),
                        tool: "hazel.recollect".to_string(),
                        command: Some(script_path),
                        args: Some(vec![]),
                        timeout_ms: Some(500),
                    }]),
                    default_policy_ids: None,
                    strict_unknown_policy_ids: Some(true),
                },
            },
            ..DaemonConfigFile::default()
        };
        let runtime = KernelRuntimeConfig::from_daemon_config(&cfg).expect("runtime config");
        let kernel = super::DaemonKernel::new(&runtime);
        let error = kernel
            .reason_submit(
                &SubmitPreparation {
                    task_id_hint: "task-000001".to_string(),
                    task_id_sequence_hint: 1,
                    session_id_hint: "session-000001".to_string(),
                    turn_id_hint: 1,
                },
                &sharo_core::protocol::SubmitTaskOpRequest {
                    session_id: Some("session-000001".to_string()),
                    goal: "explain memory".to_string(),
                    idempotency_key: None,
                },
            )
            .expect_err("semantic lint should fail");
        match error {
            sharo_core::reasoning::ReasoningError::ResolveFailure { message } => {
                assert!(message.contains("missing_provenance"));
            }
            other => panic!("unexpected error kind: {other:?}"),
        }
    }

    #[test]
    fn pre_prompt_memory_block_omits_policy_section_without_bundle() {
        let memory = super::compose_pre_prompt_memory_block(Some("plain memory"), None, &[], None);
        assert_eq!(memory, "plain memory");
    }

    #[test]
    fn pre_prompt_memory_block_includes_policy_section_without_base_memory() {
        let bundle = EffectivePolicyBundle::new(
            vec!["hunch.v1".to_string()],
            PolicyMergeMode::StrictestWins,
            vec![PolicyRule::LabelGuesses],
        );
        let memory = super::compose_pre_prompt_memory_block(None, Some(&bundle), &[], None);
        assert!(memory.contains("HAZEL_POLICY_CONTROL:"));
        assert!(memory.contains("POLICY_IDS: hunch.v1"));
        assert!(memory.contains("RULE: label guesses explicitly"));
    }
}
