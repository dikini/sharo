use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Read, Write};
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use sha2::{Digest, Sha256};
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
    EffectivePolicyBundle, HazelCardPolicyHint, HookSchemaDescriptor, ObjectSchema,
    PolicyMergeMode, PolicyRule, PrePromptComposeHookInput, RecollectionCardKind,
    RecollectionLintLimits, RecollectionPayload, ToolCallRequest, ToolCallResponse,
    expected_pre_prompt_compose_input_schema, expected_recollection_output_schema,
    input_schema_compatible, object_schema_well_formed, output_schema_compatible,
    validate_pre_prompt_compose_input_value, validate_recollection_payload_with_limits,
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

const PRE_PROMPT_TOP_K_HARD_MAX: usize = 256;
const PRE_PROMPT_TOKEN_BUDGET_HARD_MAX: usize = 65_536;
const PRE_PROMPT_TOOL_MAX_RESPONSE_BYTES: usize = 131_072;
static PRE_PROMPT_TOOL_COPY_NONCE: AtomicU64 = AtomicU64::new(0);

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
    pub pre_prompt_top_k: Option<usize>,
    pub pre_prompt_token_budget: Option<usize>,
    pub pre_prompt_relevance_threshold: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct HookBindingRuntimeConfig {
    pub id: String,
    pub tool: String,
    pub command: String,
    pub args: Vec<String>,
    pub timeout_ms: u64,
    pub command_fingerprint: BindingCommandFingerprint,
    pub schema_descriptor: HookSchemaDescriptor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingCommandFingerprint {
    #[cfg(unix)]
    pub dev: u64,
    #[cfg(unix)]
    pub inode: u64,
    #[cfg(unix)]
    pub mode: u32,
    #[cfg(unix)]
    pub uid: u32,
    #[cfg(unix)]
    pub gid: u32,
    pub content_sha256: String,
}

impl BindingCommandFingerprint {
    #[cfg(unix)]
    fn from_metadata(metadata: &fs::Metadata, content_sha256: String) -> Self {
        Self {
            dev: metadata.dev(),
            inode: metadata.ino(),
            mode: metadata.mode(),
            uid: metadata.uid(),
            gid: metadata.gid(),
            content_sha256,
        }
    }
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
        if let Some(top_k) = config.reasoning_hooks.pre_prompt_compose.top_k {
            if top_k == 0 {
                return Err("pre_prompt_compose_top_k_invalid top_k=0".to_string());
            }
            if top_k > PRE_PROMPT_TOP_K_HARD_MAX {
                return Err(format!(
                    "pre_prompt_compose_top_k_invalid top_k={} max={}",
                    top_k, PRE_PROMPT_TOP_K_HARD_MAX
                ));
            }
        }
        if let Some(token_budget) = config.reasoning_hooks.pre_prompt_compose.token_budget {
            if token_budget == 0 {
                return Err("pre_prompt_compose_token_budget_invalid token_budget=0".to_string());
            }
            if token_budget > PRE_PROMPT_TOKEN_BUDGET_HARD_MAX {
                return Err(format!(
                    "pre_prompt_compose_token_budget_invalid token_budget={} max={}",
                    token_budget, PRE_PROMPT_TOKEN_BUDGET_HARD_MAX
                ));
            }
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
            pre_prompt_top_k: config.reasoning_hooks.pre_prompt_compose.top_k,
            pre_prompt_token_budget: config.reasoning_hooks.pre_prompt_compose.token_budget,
            pre_prompt_relevance_threshold: config
                .reasoning_hooks
                .pre_prompt_compose
                .relevance_threshold,
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
    let runtime_binding = HookBindingRuntimeConfig {
        id: binding.id.clone(),
        tool: binding.tool.clone(),
        command,
        args: binding.args.clone().unwrap_or_default(),
        timeout_ms: binding.timeout_ms.unwrap_or(2_000),
        command_fingerprint: BindingCommandFingerprint {
            #[cfg(unix)]
            dev: 0,
            #[cfg(unix)]
            inode: 0,
            #[cfg(unix)]
            mode: 0,
            #[cfg(unix)]
            uid: 0,
            #[cfg(unix)]
            gid: 0,
            content_sha256: String::new(),
        },
        schema_descriptor: HookSchemaDescriptor {
            input: expected_pre_prompt_compose_input_schema(),
            output: expected_recollection_output_schema(),
        },
    };
    let mut runtime_binding = runtime_binding;
    runtime_binding.command_fingerprint = validate_binding_command_security(&runtime_binding)?;
    runtime_binding.schema_descriptor = validate_binding_schema_compatibility(&runtime_binding)?;
    Ok(Some(runtime_binding))
}

fn validate_binding_command_security(
    binding: &HookBindingRuntimeConfig,
) -> Result<BindingCommandFingerprint, String> {
    let path = Path::new(&binding.command);
    if !path.is_absolute() {
        return Err(format!(
            "pre_prompt_compose_binding_command_invalid id={} reason=path_not_absolute command={}",
            binding.id, binding.command
        ));
    }
    let mut cursor = std::path::PathBuf::new();
    for component in path.components() {
        cursor.push(component.as_os_str());
        if cursor == path {
            break;
        }
        let component_metadata = fs::symlink_metadata(&cursor).map_err(|error| {
            format!(
                "pre_prompt_compose_binding_command_invalid id={} reason=component_metadata_failed error={error}",
                binding.id
            )
        })?;
        if component_metadata.file_type().is_symlink() {
            return Err(format!(
                "pre_prompt_compose_binding_command_invalid id={} reason=parent_symlink_disallowed command={}",
                binding.id, binding.command
            ));
        }
        if component_metadata.is_dir() {
            let mode = component_metadata.mode();
            if mode & 0o002 != 0 && mode & 0o1000 == 0 {
                return Err(format!(
                    "pre_prompt_compose_binding_command_invalid id={} reason=ancestor_world_writable command={}",
                    binding.id, binding.command
                ));
            }
        }
    }
    let symlink_metadata = fs::symlink_metadata(path).map_err(|error| {
        format!(
            "pre_prompt_compose_binding_command_invalid id={} reason=symlink_metadata_failed error={error}",
            binding.id
        )
    })?;
    if symlink_metadata.file_type().is_symlink() {
        return Err(format!(
            "pre_prompt_compose_binding_command_invalid id={} reason=symlink_disallowed command={}",
            binding.id, binding.command
        ));
    }
    let metadata = fs::metadata(path).map_err(|error| {
        format!(
            "pre_prompt_compose_binding_command_invalid id={} reason=metadata_failed error={error}",
            binding.id
        )
    })?;
    if !metadata.is_file() {
        return Err(format!(
            "pre_prompt_compose_binding_command_invalid id={} reason=not_file command={}",
            binding.id, binding.command
        ));
    }
    #[cfg(unix)]
    {
        let content_sha256 = hash_file_contents(path).map_err(|error| {
            format!(
                "pre_prompt_compose_binding_command_invalid id={} reason=hash_failed error={error}",
                binding.id
            )
        })?;
        let mode = metadata.mode();
        if mode & 0o111 == 0 {
            return Err(format!(
                "pre_prompt_compose_binding_command_invalid id={} reason=not_executable command={}",
                binding.id, binding.command
            ));
        }
        if mode & 0o002 != 0 {
            return Err(format!(
                "pre_prompt_compose_binding_command_invalid id={} reason=world_writable command={}",
                binding.id, binding.command
            ));
        }
        Ok(BindingCommandFingerprint::from_metadata(
            &metadata,
            content_sha256,
        ))
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        Err("pre_prompt_compose_binding_command_invalid reason=unsupported_platform".to_string())
    }
}

fn hash_file_contents(path: &Path) -> std::io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0_u8; 16384];
    loop {
        let read = file.read(&mut buf)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn validate_binding_schema_compatibility(
    binding: &HookBindingRuntimeConfig,
) -> Result<HookSchemaDescriptor, String> {
    let request = ToolCallRequest {
        tool: "hazel.schema".to_string(),
        input: serde_json::json!({}),
    };
    let response = run_stdio_tool_call(binding, &request)?;
    if !response.ok {
        let error = response
            .error
            .unwrap_or_else(|| "unknown_schema_error".to_string());
        return Err(format!(
            "pre_prompt_compose_binding_schema_invalid id={} reason=schema_tool_failed error_hint={}",
            binding.id,
            escape_log_field(&stderr_error_hint(&error))
        ));
    }
    let output = response.output.ok_or_else(|| {
        format!(
            "pre_prompt_compose_binding_schema_invalid id={} reason=schema_output_missing",
            binding.id
        )
    })?;
    let descriptor: HookSchemaDescriptor = serde_json::from_value(output).map_err(|error| {
        format!(
            "pre_prompt_compose_binding_schema_invalid id={} reason=schema_parse_failed error={error}",
            binding.id
        )
    })?;
    if !object_schema_well_formed(&descriptor.input) {
        return Err(format!(
            "pre_prompt_compose_binding_schema_invalid id={} reason=input_schema_malformed",
            binding.id
        ));
    }
    if !object_schema_well_formed(&descriptor.output) {
        return Err(format!(
            "pre_prompt_compose_binding_schema_invalid id={} reason=output_schema_malformed",
            binding.id
        ));
    }
    let expected_input = expected_pre_prompt_compose_input_schema();
    let expected_output = expected_recollection_output_schema();
    if !input_schema_compatible(&expected_input, &descriptor.input) {
        return Err(format!(
            "pre_prompt_compose_binding_schema_invalid id={} reason=input_incompatible",
            binding.id
        ));
    }
    if !output_schema_compatible(&expected_output, &descriptor.output) {
        return Err(format!(
            "pre_prompt_compose_binding_schema_invalid id={} reason=output_incompatible",
            binding.id
        ));
    }
    Ok(descriptor)
}

fn validate_payload_against_object_schema(
    value: &serde_json::Value,
    schema: &ObjectSchema,
    error_prefix: &str,
) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{error_prefix} reason=not_object"))?;
    for required in &schema.required {
        if !object.contains_key(required) {
            return Err(format!(
                "{error_prefix} reason=missing_required_field field={required}"
            ));
        }
    }
    if !schema.allow_additional {
        for key in object.keys() {
            if !schema.allowed.contains(key) {
                return Err(format!("{error_prefix} reason=unknown_field field={key}"));
            }
        }
    }
    Ok(())
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
    top_k: Option<usize>,
    token_budget: Option<usize>,
    relevance_threshold: Option<f32>,
}

impl PrePromptComposeMemoryResolver {
    fn emit_hook_failure_event(
        &self,
        binding_id: &str,
        task_id: &str,
        started: Instant,
        error: &str,
    ) {
        let error_hint = stderr_error_hint(error);
        eprintln!(
            "hazel_hook event=tool_failed binding={} task_id={} elapsed_ms={} error_hint={}",
            binding_id,
            task_id,
            started.elapsed().as_millis(),
            escape_log_field(&error_hint)
        );
    }

    fn recollection_lint_limits(&self) -> RecollectionLintLimits {
        let defaults = RecollectionLintLimits::default();
        let max_cards = self.top_k.unwrap_or(defaults.max_cards).max(1);
        let max_tokens = self.token_budget.unwrap_or(defaults.max_tokens).max(1);
        RecollectionLintLimits {
            max_cards,
            max_payload_bytes: defaults.max_payload_bytes,
            max_tokens,
        }
    }

    fn run_pre_prompt_hook(
        &self,
        scope: &TurnScope,
    ) -> Result<Option<RecollectionPayload>, String> {
        let Some(binding) = &self.hook_binding else {
            return Ok(None);
        };
        let started = Instant::now();
        let input = PrePromptComposeHookInput {
            session_id: scope.session_id.clone(),
            task_id: scope.task_id.clone(),
            goal: scope.goal.clone(),
            runtime: self
                .runtime_context
                .clone()
                .unwrap_or_else(|| "daemon".to_string()),
            top_k: self.top_k,
            token_budget: self.token_budget,
            relevance_threshold: self.relevance_threshold,
            policy_ids: self
                .effective_policy
                .as_ref()
                .map(|bundle| bundle.effective_policy_ids.clone())
                .unwrap_or_default(),
            card_policy_hints: self.card_policy_hints.clone(),
        };
        let input_value = serde_json::to_value(input)
            .map_err(|error| format!("pre_prompt_input_encode_failed error={error}"))?;
        validate_pre_prompt_compose_input_value(&input_value)?;
        let request = ToolCallRequest {
            tool: binding.tool.clone(),
            input: input_value,
        };
        validate_payload_against_object_schema(
            &request.input,
            &binding.schema_descriptor.input,
            "pre_prompt_input_schema_invalid",
        )?;
        let response = match run_stdio_tool_call(binding, &request) {
            Ok(response) => response,
            Err(error) => {
                self.emit_hook_failure_event(&binding.id, &scope.task_id, started, &error);
                return Err(error);
            }
        };
        if !response.ok {
            let error = response
                .error
                .unwrap_or_else(|| "pre_prompt_tool_error_unknown".to_string());
            self.emit_hook_failure_event(&binding.id, &scope.task_id, started, &error);
            let error_hint = stderr_error_hint(&error);
            return Err(format!(
                "pre_prompt_tool_failed binding={} error_hint={}",
                binding.id, error_hint
            ));
        }
        let Some(output) = response.output else {
            let error = format!("pre_prompt_tool_output_missing binding={}", binding.id);
            self.emit_hook_failure_event(&binding.id, &scope.task_id, started, &error);
            return Err(format!(
                "pre_prompt_tool_output_missing binding={}",
                binding.id
            ));
        };
        if let Err(error) = validate_payload_against_object_schema(
            &output,
            &binding.schema_descriptor.output,
            "pre_prompt_output_schema_invalid",
        ) {
            self.emit_hook_failure_event(&binding.id, &scope.task_id, started, &error);
            return Err(error);
        }
        let recollection = match validate_recollection_payload_with_limits(
            &output,
            &self.recollection_lint_limits(),
        ) {
            Ok(recollection) => recollection,
            Err(error) => {
                self.emit_hook_failure_event(&binding.id, &scope.task_id, started, &error);
                return Err(error);
            }
        };
        eprintln!(
            "hazel_hook event=tool_succeeded binding={} task_id={} elapsed_ms={} cards={}",
            binding.id,
            scope.task_id,
            started.elapsed().as_millis(),
            recollection.cards.len()
        );
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
    let mut terminal_error = None;
    for attempt in 0..2_u8 {
        let prepared_command = prepare_verified_binding_command(binding)?;
        let spawn = Command::new(&prepared_command.path)
            .args(&binding.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();
        match spawn {
            Ok(child) => {
                return run_stdio_tool_call_with_child(binding, request, child, prepared_command);
            }
            Err(error) if error.raw_os_error() == Some(26) && attempt == 0 => {
                drop(prepared_command);
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => {
                drop(prepared_command);
                terminal_error = Some(error);
            }
        }
    }
    let error = terminal_error.unwrap_or_else(|| std::io::Error::other("unknown_spawn_error"));
    Err(format!(
        "pre_prompt_tool_spawn_failed binding={} command={} error={error}",
        binding.id, binding.command
    ))
}

fn verify_binding_command_integrity(
    binding: &HookBindingRuntimeConfig,
) -> Result<BindingCommandFingerprint, String> {
    let current = validate_binding_command_security(binding).map_err(|error| {
        format!(
            "pre_prompt_tool_binding_command_invalid binding={} error={}",
            binding.id, error
        )
    })?;
    if current != binding.command_fingerprint {
        return Err(format!(
            "pre_prompt_tool_binding_command_invalid binding={} reason=command_identity_mismatch",
            binding.id
        ));
    }
    Ok(current)
}

#[derive(Debug)]
struct PreparedBindingCommand {
    path: PathBuf,
}

impl Drop for PreparedBindingCommand {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn prepare_verified_binding_command(
    binding: &HookBindingRuntimeConfig,
) -> Result<PreparedBindingCommand, String> {
    let (bytes, content_sha256) = read_file_bytes_and_sha256(Path::new(&binding.command))?;
    if content_sha256 != binding.command_fingerprint.content_sha256 {
        return Err(format!(
            "pre_prompt_tool_binding_command_invalid binding={} reason=command_identity_mismatch",
            binding.id
        ));
    }
    let _ = verify_binding_command_integrity(binding)?;
    let copy_path = unique_verified_command_path(binding);
    let mut copy_file = open_restricted_copy_file(&copy_path).map_err(|error| {
        format!(
            "pre_prompt_tool_binding_command_invalid binding={} reason=copy_create_failed error={error}",
            binding.id
        )
    })?;
    copy_file.write_all(&bytes).map_err(|error| {
        format!(
            "pre_prompt_tool_binding_command_invalid binding={} reason=copy_write_failed error={error}",
            binding.id
        )
    })?;
    copy_file.sync_all().map_err(|error| {
        format!(
            "pre_prompt_tool_binding_command_invalid binding={} reason=copy_sync_failed error={error}",
            binding.id
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&copy_path, fs::Permissions::from_mode(0o700)).map_err(|error| {
            format!(
                "pre_prompt_tool_binding_command_invalid binding={} reason=copy_chmod_failed error={error}",
                binding.id
            )
        })?;
    }
    Ok(PreparedBindingCommand { path: copy_path })
}

fn unique_verified_command_path(binding: &HookBindingRuntimeConfig) -> PathBuf {
    let nonce = PRE_PROMPT_TOOL_COPY_NONCE.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let safe_binding_id = sanitize_binding_id_for_path(&binding.id);
    std::env::temp_dir().join(format!(
        "sharo-hazel-hook-{}-{}-{nonce}.bin",
        safe_binding_id, pid
    ))
}

fn sanitize_binding_id_for_path(id: &str) -> String {
    let mut sanitized = String::new();
    for ch in id.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
        }
    }
    if sanitized.is_empty() {
        return "binding".to_string();
    }
    if sanitized.len() > 64 {
        sanitized.truncate(64);
    }
    sanitized
}

fn read_file_bytes_and_sha256(path: &Path) -> Result<(Vec<u8>, String), String> {
    let bytes = fs::read(path).map_err(|error| {
        format!("pre_prompt_tool_binding_command_invalid reason=read_failed error={error}")
    })?;
    let digest = Sha256::digest(&bytes);
    Ok((bytes, format!("{digest:x}")))
}

fn run_stdio_tool_call_with_child(
    binding: &HookBindingRuntimeConfig,
    request: &ToolCallRequest,
    mut child: std::process::Child,
    _prepared_command: PreparedBindingCommand,
) -> Result<ToolCallResponse, String> {
    {
        let Some(mut stdin) = child.stdin.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "pre_prompt_tool_stdin_unavailable binding={}",
                binding.id
            ));
        };
        let payload = match serde_json::to_string(request) {
            Ok(payload) => payload,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "pre_prompt_tool_request_encode_failed error={error}"
                ));
            }
        };
        if let Err(error) = writeln!(stdin, "{payload}") {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "pre_prompt_tool_request_write_failed error={error}"
            ));
        }
    }

    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(format!(
            "pre_prompt_tool_stdout_unavailable binding={}",
            binding.id
        ));
    };
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut reader =
            BufReader::new(stdout.take((PRE_PROMPT_TOOL_MAX_RESPONSE_BYTES + 1) as u64));
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
    if let Err(error) = read_result {
        let _ = child.kill();
        let _ = child.wait();
        return Err(format!(
            "pre_prompt_tool_response_read_failed error={error}"
        ));
    }
    if line.len() > PRE_PROMPT_TOOL_MAX_RESPONSE_BYTES {
        let _ = child.kill();
        let _ = child.wait();
        return Err(format!(
            "pre_prompt_tool_response_too_large binding={} max_bytes={}",
            binding.id, PRE_PROMPT_TOOL_MAX_RESPONSE_BYTES
        ));
    }

    let status = child
        .wait()
        .map_err(|error| format!("pre_prompt_tool_wait_failed error={error}"))?;
    if !status.success() {
        let error_hint = match status.code() {
            Some(code) => format!("tool_error;exit_code={code}"),
            None => "tool_error;terminated_by_signal".to_string(),
        };
        return Err(format!(
            "pre_prompt_tool_nonzero_exit binding={} status={} error_hint={}",
            binding.id, status, error_hint
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

fn open_restricted_copy_file(path: &Path) -> std::io::Result<fs::File> {
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o700);
    }
    options.open(path)
}

fn escape_log_field(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"<encode_failed>\"".to_string())
}

fn stderr_error_hint(stderr: &str) -> String {
    if stderr.is_empty() {
        return "none".to_string();
    }
    let lower = stderr.to_ascii_lowercase();
    let has_timeout = lower.contains("timeout");
    let has_auth = lower.contains("auth");
    let has_permission = lower.contains("permission");
    let has_json = lower.contains("json");
    let class = if has_auth {
        "auth"
    } else if has_permission {
        "permission"
    } else if has_json {
        "json"
    } else if has_timeout {
        "timeout"
    } else {
        "tool_error"
    };
    format!("{class};len={}", stderr.len())
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
    pre_prompt_top_k: Option<usize>,
    pre_prompt_token_budget: Option<usize>,
    pre_prompt_relevance_threshold: Option<f32>,
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
                top_k: config.pre_prompt_top_k,
                token_budget: config.pre_prompt_token_budget,
                relevance_threshold: config.pre_prompt_relevance_threshold,
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
            pre_prompt_top_k: config.pre_prompt_top_k,
            pre_prompt_token_budget: config.pre_prompt_token_budget,
            pre_prompt_relevance_threshold: config.pre_prompt_relevance_threshold,
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
        if let Some(value) = self.pre_prompt_top_k {
            metadata.insert("hook.pre_prompt.top_k".to_string(), value.to_string());
        }
        if let Some(value) = self.pre_prompt_token_budget {
            metadata.insert(
                "hook.pre_prompt.token_budget".to_string(),
                value.to_string(),
            );
        }
        if let Some(value) = self.pre_prompt_relevance_threshold {
            metadata.insert(
                "hook.pre_prompt.relevance_threshold".to_string(),
                value.to_string(),
            );
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
    #[cfg(unix)]
    use std::os::unix::fs::symlink;
    use std::path::PathBuf;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{ConnectorKind, KernelRuntimeConfig};
    use crate::config::{
        ConnectorPoolConfig, DaemonConfigFile, HookBindingConfig, HookPolicyDefinitionConfig,
        ModelRuntimeConfig, PrePromptComposeHookConfig, ReasoningHooksConfig,
    };
    use crate::store::SubmitPreparation;
    use sharo_core::protocol::{
        EffectivePolicyBundle, HookSchemaDescriptor, PolicyMergeMode, PolicyRule,
        expected_pre_prompt_compose_input_schema, expected_recollection_output_schema,
    };
    use sharo_core::reasoning_context::PolicyConfig;

    fn default_mock_schema_response() -> String {
        let descriptor = HookSchemaDescriptor {
            input: expected_pre_prompt_compose_input_schema(),
            output: expected_recollection_output_schema(),
        };
        serde_json::to_string(&serde_json::json!({
            "ok": true,
            "output": descriptor,
            "error": serde_json::Value::Null
        }))
        .expect("serialize schema response")
    }

    fn write_mock_mcp_script_with_schema(
        recollect_response_line: &str,
        schema_response_line: &str,
    ) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        write_mock_mcp_script_with_schema_at(
            std::env::temp_dir(),
            &format!("sharo-hazel-mcp-mock-{nanos}.sh"),
            recollect_response_line,
            schema_response_line,
        )
    }

    fn write_mock_mcp_script_with_schema_at(
        directory: PathBuf,
        file_name: &str,
        recollect_response_line: &str,
        schema_response_line: &str,
    ) -> String {
        let path = directory.join(file_name);
        let content = format!(
            "#!/usr/bin/env bash\nread _line\nif [[ \"$_line\" == *'\"tool\":\"hazel.schema\"'* ]]; then\n  echo '{}'\nelse\n  echo '{}'\nfi\n",
            schema_response_line.replace('\'', "'\"'\"'"),
            recollect_response_line.replace('\'', "'\"'\"'")
        );
        fs::write(&path, content).expect("write mock mcp script");
        let mut permissions = fs::metadata(&path).expect("metadata").permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&path, permissions).expect("chmod script");
        path.to_string_lossy().to_string()
    }

    fn write_mock_mcp_script(response_line: &str) -> String {
        write_mock_mcp_script_with_schema(response_line, &default_mock_schema_response())
    }

    fn ensure_real_hazel_mcp_binary() -> String {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("crates dir")
            .parent()
            .expect("workspace root")
            .to_path_buf();
        let status = Command::new("cargo")
            .arg("build")
            .arg("-p")
            .arg("sharo-hazel-mcp")
            .current_dir(&workspace_root)
            .status()
            .expect("build sharo-hazel-mcp");
        assert!(status.success(), "sharo-hazel-mcp build must succeed");
        let bin_path = workspace_root
            .join("target")
            .join("debug")
            .join("sharo-hazel-mcp");
        assert!(bin_path.exists(), "expected hazel mcp binary");
        bin_path.to_string_lossy().to_string()
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
    fn authenticated_loopback_http_base_url_remains_allowed_for_local_tests() {
        authenticated_provider_allows_loopback_http_base_url();
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
                    top_k: None,
                    token_budget: None,
                    relevance_threshold: None,
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
                    top_k: None,
                    token_budget: None,
                    relevance_threshold: None,
                },
            },
            ..DaemonConfigFile::default()
        };
        let err = KernelRuntimeConfig::from_daemon_config(&cfg)
            .expect_err("unknown strict policy id should fail");
        assert!(err.contains("pre_prompt_compose_policy_missing"));
    }

    #[test]
    fn pre_prompt_compose_rejects_relative_binding_command() {
        let cfg = DaemonConfigFile {
            reasoning_hooks: ReasoningHooksConfig {
                pre_prompt_compose: PrePromptComposeHookConfig {
                    composition: Some("single".to_string()),
                    bindings: Some(vec![HookBindingConfig {
                        id: "hazel".to_string(),
                        tool: "hazel.recollect".to_string(),
                        command: Some("hazel-mcp".to_string()),
                        args: Some(vec![]),
                        timeout_ms: Some(500),
                    }]),
                    default_policy_ids: None,
                    strict_unknown_policy_ids: Some(true),
                    top_k: None,
                    token_budget: None,
                    relevance_threshold: None,
                },
            },
            ..DaemonConfigFile::default()
        };
        let err = KernelRuntimeConfig::from_daemon_config(&cfg)
            .expect_err("relative command paths must be rejected");
        assert!(err.contains("path_not_absolute"));
    }

    #[test]
    fn pre_prompt_compose_rejects_world_writable_binding_command() {
        let script_path = write_mock_mcp_script(
            r#"{"ok":true,"output":{"policy_ids":["hunch.v1"],"cards":[{"card_id":"card-1","kind":"association_cue","state":"candidate","subject":"memory-system","text":"Use prior architecture context","provenance":[{"source_ref":"note:hazel"}],"policy_ids":["hunch.v1"]}]}}"#,
        );
        let mut permissions = fs::metadata(&script_path).expect("metadata").permissions();
        permissions.set_mode(0o777);
        fs::set_permissions(&script_path, permissions).expect("chmod script world writable");
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
                    top_k: None,
                    token_budget: None,
                    relevance_threshold: None,
                },
            },
            ..DaemonConfigFile::default()
        };
        let err = KernelRuntimeConfig::from_daemon_config(&cfg)
            .expect_err("world-writable commands must be rejected");
        assert!(err.contains("world_writable"));
    }

    #[test]
    fn pre_prompt_compose_rejects_world_writable_parent_directory() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let parent = std::env::temp_dir().join(format!("sharo-hazel-mcp-parent-{nanos}"));
        fs::create_dir_all(&parent).expect("create parent dir");
        let mut parent_permissions = fs::metadata(&parent).expect("metadata").permissions();
        parent_permissions.set_mode(0o777);
        fs::set_permissions(&parent, parent_permissions).expect("chmod parent world writable");
        let script_path = write_mock_mcp_script_with_schema_at(
            parent.clone(),
            "mock.sh",
            r#"{"ok":true,"output":{"policy_ids":["hunch.v1"],"cards":[{"card_id":"card-1","kind":"association_cue","state":"candidate","subject":"memory-system","text":"Use prior architecture context","provenance":[{"source_ref":"note:hazel"}],"policy_ids":["hunch.v1"]}]}}"#,
            &default_mock_schema_response(),
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
                    top_k: None,
                    token_budget: None,
                    relevance_threshold: None,
                },
            },
            ..DaemonConfigFile::default()
        };
        let err = KernelRuntimeConfig::from_daemon_config(&cfg)
            .expect_err("world-writable parent must be rejected");
        assert!(err.contains("ancestor_world_writable"));
        let mut restore = fs::metadata(&parent).expect("metadata").permissions();
        restore.set_mode(0o700);
        fs::set_permissions(&parent, restore).expect("restore parent permissions");
        fs::remove_file(parent.join("mock.sh")).expect("remove mock script");
        fs::remove_dir(&parent).expect("remove parent dir");
    }

    #[test]
    #[cfg(unix)]
    fn pre_prompt_compose_rejects_symlinked_parent_component() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("sharo-hazel-mcp-symlink-{nanos}"));
        let real_parent = root.join("real");
        let link_parent = root.join("link");
        fs::create_dir_all(&real_parent).expect("create real parent");
        symlink(&real_parent, &link_parent).expect("create symlinked parent");
        let script_path = write_mock_mcp_script_with_schema_at(
            real_parent.clone(),
            "mock.sh",
            r#"{"ok":true,"output":{"policy_ids":["hunch.v1"],"cards":[{"card_id":"card-1","kind":"association_cue","state":"candidate","subject":"memory-system","text":"Use prior architecture context","provenance":[{"source_ref":"note:hazel"}],"policy_ids":["hunch.v1"]}]}}"#,
            &default_mock_schema_response(),
        );
        let symlink_script_path = link_parent.join("mock.sh");
        assert!(PathBuf::from(&script_path).exists());
        let cfg = DaemonConfigFile {
            reasoning_hooks: ReasoningHooksConfig {
                pre_prompt_compose: PrePromptComposeHookConfig {
                    composition: Some("single".to_string()),
                    bindings: Some(vec![HookBindingConfig {
                        id: "hazel".to_string(),
                        tool: "hazel.recollect".to_string(),
                        command: Some(symlink_script_path.to_string_lossy().to_string()),
                        args: Some(vec![]),
                        timeout_ms: Some(500),
                    }]),
                    default_policy_ids: None,
                    strict_unknown_policy_ids: Some(true),
                    top_k: None,
                    token_budget: None,
                    relevance_threshold: None,
                },
            },
            ..DaemonConfigFile::default()
        };
        let err = KernelRuntimeConfig::from_daemon_config(&cfg)
            .expect_err("symlinked parent path component must be rejected");
        assert!(err.contains("parent_symlink_disallowed"));
        fs::remove_file(real_parent.join("mock.sh")).expect("remove mock script");
        fs::remove_file(&link_parent).expect("remove symlink");
        fs::remove_dir(&real_parent).expect("remove real parent");
        fs::remove_dir(&root).expect("remove root");
    }

    #[test]
    fn pre_prompt_compose_rejects_top_k_above_hard_limit() {
        let cfg = DaemonConfigFile {
            reasoning_hooks: ReasoningHooksConfig {
                pre_prompt_compose: PrePromptComposeHookConfig {
                    composition: Some("single".to_string()),
                    bindings: None,
                    default_policy_ids: None,
                    strict_unknown_policy_ids: Some(true),
                    top_k: Some(super::PRE_PROMPT_TOP_K_HARD_MAX + 1),
                    token_budget: None,
                    relevance_threshold: None,
                },
            },
            ..DaemonConfigFile::default()
        };
        let err = KernelRuntimeConfig::from_daemon_config(&cfg)
            .expect_err("top_k above hard max must fail");
        assert!(err.contains("pre_prompt_compose_top_k_invalid"));
    }

    #[test]
    fn pre_prompt_compose_rejects_token_budget_above_hard_limit() {
        let cfg = DaemonConfigFile {
            reasoning_hooks: ReasoningHooksConfig {
                pre_prompt_compose: PrePromptComposeHookConfig {
                    composition: Some("single".to_string()),
                    bindings: None,
                    default_policy_ids: None,
                    strict_unknown_policy_ids: Some(true),
                    top_k: None,
                    token_budget: Some(super::PRE_PROMPT_TOKEN_BUDGET_HARD_MAX + 1),
                    relevance_threshold: None,
                },
            },
            ..DaemonConfigFile::default()
        };
        let err = KernelRuntimeConfig::from_daemon_config(&cfg)
            .expect_err("token_budget above hard max must fail");
        assert!(err.contains("pre_prompt_compose_token_budget_invalid"));
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
                    top_k: None,
                    token_budget: None,
                    relevance_threshold: None,
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
                    top_k: None,
                    token_budget: None,
                    relevance_threshold: None,
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
                    top_k: None,
                    token_budget: None,
                    relevance_threshold: None,
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
                    top_k: None,
                    token_budget: None,
                    relevance_threshold: None,
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
    fn pre_prompt_compose_with_real_hazel_mcp_binary_injects_cards() {
        let mcp_bin = ensure_real_hazel_mcp_binary();
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
                        command: Some(mcp_bin),
                        args: Some(vec![]),
                        timeout_ms: Some(1_500),
                    }]),
                    default_policy_ids: Some(vec!["hunch.v1".to_string()]),
                    strict_unknown_policy_ids: Some(true),
                    top_k: Some(1),
                    token_budget: Some(256),
                    relevance_threshold: Some(0.0),
                },
            },
            hook_policies,
            hazel_manifest: crate::config::HazelManifestConfig {
                cards: vec![crate::config::HazelCardManifestConfig {
                    kind: "association_cue".to_string(),
                    policy_ids: vec!["hunch.v1".to_string()],
                    max_cards: Some(1),
                }],
            },
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
                    goal: "memory subsystem architecture".to_string(),
                    idempotency_key: None,
                },
            )
            .expect("reasoning outcome");
        assert!(outcome.model_output_text.contains("HAZEL_RECOLLECTIONS:"));
        assert!(
            outcome
                .model_output_text
                .contains("structured memory subsystem for sharo")
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
                    top_k: None,
                    token_budget: None,
                    relevance_threshold: None,
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
    fn pre_prompt_compose_rejects_schema_handshake_incompatible_output() {
        let incompatible_schema = serde_json::to_string(&serde_json::json!({
            "ok": true,
            "output": {
                "input": {
                    "required": ["session_id", "task_id", "goal", "runtime"],
                    "allowed": ["session_id", "task_id", "goal", "runtime"],
                    "allow_additional": false
                },
                "output": {
                    "required": ["policy_ids", "cards"],
                    "allowed": ["policy_ids", "cards", "extra_field"],
                    "allow_additional": false
                }
            },
            "error": serde_json::Value::Null
        }))
        .expect("serialize incompatible schema response");
        let script_path = write_mock_mcp_script_with_schema(
            r#"{"ok":true,"output":{"policy_ids":["hunch.v1"],"cards":[{"card_id":"card-1","kind":"association_cue","state":"candidate","subject":"memory-system","text":"Use prior architecture context","provenance":[{"source_ref":"note:hazel"}],"policy_ids":["hunch.v1"]}]}}"#,
            &incompatible_schema,
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
                    top_k: None,
                    token_budget: None,
                    relevance_threshold: None,
                },
            },
            ..DaemonConfigFile::default()
        };
        let err = KernelRuntimeConfig::from_daemon_config(&cfg)
            .expect_err("incompatible schema should fail startup validation");
        assert!(err.contains("pre_prompt_compose_binding_schema_invalid"));
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
                    top_k: None,
                    token_budget: None,
                    relevance_threshold: None,
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
    fn pre_prompt_compose_rejects_recollections_exceeding_top_k_limit() {
        let script_path = write_mock_mcp_script(
            r#"{"ok":true,"output":{"policy_ids":["hunch.v1"],"cards":[{"card_id":"card-1","kind":"association_cue","state":"candidate","subject":"x","text":"x","provenance":[{"source_ref":"a"}],"policy_ids":["hunch.v1"]},{"card_id":"card-2","kind":"association_cue","state":"candidate","subject":"y","text":"y","provenance":[{"source_ref":"b"}],"policy_ids":["hunch.v1"]}]}}"#,
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
                    top_k: Some(1),
                    token_budget: None,
                    relevance_threshold: None,
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
            .expect_err("top_k overflow should fail semantic lint");
        match error {
            sharo_core::reasoning::ReasoningError::ResolveFailure { message } => {
                assert!(message.contains("max_cards_exceeded"));
            }
            other => panic!("unexpected error kind: {other:?}"),
        }
    }

    #[test]
    fn pre_prompt_compose_rejects_tool_response_exceeding_transport_limit() {
        let huge_line = "x".repeat(super::PRE_PROMPT_TOOL_MAX_RESPONSE_BYTES + 64);
        let script_path = write_mock_mcp_script(&huge_line);
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
                    top_k: None,
                    token_budget: None,
                    relevance_threshold: None,
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
            .expect_err("oversized tool response should fail");
        match error {
            sharo_core::reasoning::ReasoningError::ResolveFailure { message } => {
                assert!(message.contains("response_too_large"));
            }
            other => panic!("unexpected error kind: {other:?}"),
        }
    }

    #[test]
    fn pre_prompt_compose_rejects_binding_command_identity_drift_at_runtime() {
        let script_path = write_mock_mcp_script(
            r#"{"ok":true,"output":{"policy_ids":["hunch.v1"],"cards":[{"card_id":"card-1","kind":"association_cue","state":"candidate","subject":"memory-system","text":"Use prior architecture context","provenance":[{"source_ref":"note:hazel"}],"policy_ids":["hunch.v1"]}]}}"#,
        );
        let cfg = DaemonConfigFile {
            reasoning_hooks: ReasoningHooksConfig {
                pre_prompt_compose: PrePromptComposeHookConfig {
                    composition: Some("single".to_string()),
                    bindings: Some(vec![HookBindingConfig {
                        id: "hazel".to_string(),
                        tool: "hazel.recollect".to_string(),
                        command: Some(script_path.clone()),
                        args: Some(vec![]),
                        timeout_ms: Some(500),
                    }]),
                    default_policy_ids: None,
                    strict_unknown_policy_ids: Some(true),
                    top_k: None,
                    token_budget: None,
                    relevance_threshold: None,
                },
            },
            ..DaemonConfigFile::default()
        };
        let runtime = KernelRuntimeConfig::from_daemon_config(&cfg).expect("runtime config");
        let kernel = super::DaemonKernel::new(&runtime);

        let replacement_path = PathBuf::from(&script_path).with_file_name("replacement-mock.sh");
        let replacement = write_mock_mcp_script_with_schema_at(
            replacement_path
                .parent()
                .expect("replacement parent")
                .to_path_buf(),
            replacement_path
                .file_name()
                .expect("replacement filename")
                .to_string_lossy()
                .as_ref(),
            r#"{"ok":true,"output":{"policy_ids":["hunch.v1"],"cards":[{"card_id":"card-2","kind":"association_cue","state":"candidate","subject":"memory-system","text":"Replacement script output","provenance":[{"source_ref":"note:hazel"}],"policy_ids":["hunch.v1"]}]}}"#,
            &default_mock_schema_response(),
        );
        fs::rename(&replacement, &script_path).expect("replace validated script");

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
            .expect_err("runtime command identity drift should fail");
        match error {
            sharo_core::reasoning::ReasoningError::ResolveFailure { message } => {
                assert!(
                    message.contains("command_identity_mismatch")
                        || (message.contains("pre_prompt_tool_spawn_failed")
                            && message.contains("Text file busy")),
                    "unexpected in-place mutation failure mode: {message}"
                );
            }
            other => panic!("unexpected error kind: {other:?}"),
        }
    }

    #[test]
    fn pre_prompt_compose_rejects_in_place_binding_command_content_mutation() {
        let script_path = write_mock_mcp_script(
            r#"{"ok":true,"output":{"policy_ids":["hunch.v1"],"cards":[{"card_id":"card-1","kind":"association_cue","state":"candidate","subject":"memory-system","text":"Use prior architecture context","provenance":[{"source_ref":"note:hazel"}],"policy_ids":["hunch.v1"]}]}}"#,
        );
        let cfg = DaemonConfigFile {
            reasoning_hooks: ReasoningHooksConfig {
                pre_prompt_compose: PrePromptComposeHookConfig {
                    composition: Some("single".to_string()),
                    bindings: Some(vec![HookBindingConfig {
                        id: "hazel".to_string(),
                        tool: "hazel.recollect".to_string(),
                        command: Some(script_path.clone()),
                        args: Some(vec![]),
                        timeout_ms: Some(500),
                    }]),
                    default_policy_ids: None,
                    strict_unknown_policy_ids: Some(true),
                    top_k: None,
                    token_budget: None,
                    relevance_threshold: None,
                },
            },
            ..DaemonConfigFile::default()
        };
        let runtime = KernelRuntimeConfig::from_daemon_config(&cfg).expect("runtime config");
        let kernel = super::DaemonKernel::new(&runtime);

        let replacement_content = format!(
            "#!/usr/bin/env bash\nread _line\nif [[ \"$_line\" == *'\"tool\":\"hazel.schema\"'* ]]; then\n  echo '{}'\nelse\n  echo '{}'\nfi\n",
            default_mock_schema_response().replace('\'', "'\"'\"'"),
            r#"{"ok":true,"output":{"policy_ids":["hunch.v1"],"cards":[{"card_id":"card-x","kind":"association_cue","state":"candidate","subject":"memory-system","text":"Mutated in-place output","provenance":[{"source_ref":"note:hazel"}],"policy_ids":["hunch.v1"]}]}}"#
                .replace('\'', "'\"'\"'")
        );
        fs::write(&script_path, replacement_content).expect("mutate script content in-place");

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
            .expect_err("in-place command content mutation should fail");
        match error {
            sharo_core::reasoning::ReasoningError::ResolveFailure { message } => {
                assert!(message.contains("command_identity_mismatch"));
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

    #[test]
    fn pre_prompt_compose_rejects_world_writable_ancestor_directory() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let ancestor = std::env::temp_dir().join(format!("sharo-hazel-mcp-ancestor-{nanos}"));
        let nested = ancestor.join("safe");
        fs::create_dir_all(&nested).expect("create nested parent dirs");
        let mut ancestor_permissions = fs::metadata(&ancestor).expect("metadata").permissions();
        ancestor_permissions.set_mode(0o777);
        fs::set_permissions(&ancestor, ancestor_permissions)
            .expect("chmod ancestor world writable");
        let script_path = write_mock_mcp_script_with_schema_at(
            nested,
            "mock.sh",
            r#"{"ok":true,"output":{"policy_ids":["hunch.v1"],"cards":[{"card_id":"card-1","kind":"association_cue","state":"candidate","subject":"memory-system","text":"Use prior architecture context","provenance":[{"source_ref":"note:hazel"}],"policy_ids":["hunch.v1"]}]}}"#,
            &default_mock_schema_response(),
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
                    top_k: None,
                    token_budget: None,
                    relevance_threshold: None,
                },
            },
            ..DaemonConfigFile::default()
        };
        let err = KernelRuntimeConfig::from_daemon_config(&cfg)
            .expect_err("world-writable ancestor directories must be rejected");
        assert!(err.contains("ancestor_world_writable"));
    }

    #[test]
    fn sanitize_binding_id_for_path_replaces_disallowed_characters() {
        let sanitized = super::sanitize_binding_id_for_path("../hazel alpha.beta\tgamma");
        assert_eq!(sanitized, "___hazel_alpha_beta_gamma");
        assert!(
            sanitized
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
        );
    }

    #[test]
    fn unique_verified_command_path_uses_sanitized_binding_id() {
        let script_path =
            write_mock_mcp_script(r#"{"ok":true,"output":{"policy_ids":["hunch.v1"],"cards":[]}}"#);
        let cfg = DaemonConfigFile {
            reasoning_hooks: ReasoningHooksConfig {
                pre_prompt_compose: PrePromptComposeHookConfig {
                    composition: Some("single".to_string()),
                    bindings: Some(vec![HookBindingConfig {
                        id: "../hazel alpha.beta".to_string(),
                        tool: "hazel.recollect".to_string(),
                        command: Some(script_path),
                        args: Some(vec![]),
                        timeout_ms: Some(500),
                    }]),
                    default_policy_ids: None,
                    strict_unknown_policy_ids: Some(true),
                    top_k: None,
                    token_budget: None,
                    relevance_threshold: None,
                },
            },
            ..DaemonConfigFile::default()
        };
        let runtime = KernelRuntimeConfig::from_daemon_config(&cfg).expect("runtime config");
        let binding = runtime
            .pre_prompt_hook_binding
            .as_ref()
            .expect("hook binding");
        let path = super::unique_verified_command_path(binding);
        let file_name = path
            .file_name()
            .expect("file name")
            .to_string_lossy()
            .to_string();
        assert!(file_name.starts_with("sharo-hazel-hook-___hazel_alpha_beta-"));
        assert!(!file_name.contains('/'));
        assert!(!file_name.contains('\\'));
        assert!(!file_name.contains(' '));
        assert!(!file_name.contains(".."));
    }
}
