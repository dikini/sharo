use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};

use crate::reasoning::ReasoningInput;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnScope {
    pub session_id: String,
    pub task_id: String,
    pub turn_id: u64,
    pub goal: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposePrompt {
    pub prompt_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextState {
    pub system: String,
    pub persona: String,
    pub memory: String,
    pub runtime: String,
    pub goal: String,
}

impl ContextState {
    pub fn default_with_goal(goal: String) -> Self {
        Self {
            system: String::new(),
            persona: String::new(),
            memory: String::new(),
            runtime: String::new(),
            goal,
        }
    }

    pub fn from_reasoning_input_defaults(input: &ReasoningInput) -> Self {
        Self::default_with_goal(input.goal.clone())
    }

    pub fn state_hash(&self) -> String {
        let payload = serde_json::to_vec(&(
            self.system.as_str(),
            self.persona.as_str(),
            self.memory.as_str(),
            self.runtime.as_str(),
            self.goal.as_str(),
        ))
        .unwrap_or_else(|_| {
            format!(
                "system_len={} persona_len={} memory_len={} runtime_len={} goal_len={}",
                self.system.len(),
                self.persona.len(),
                self.memory.len(),
                self.runtime.len(),
                self.goal.len()
            )
            .into_bytes()
        });
        let digest = Sha256::digest(payload);
        let mut out = String::with_capacity(digest.len() * 2);
        for byte in digest {
            use std::fmt::Write as _;
            let _ = write!(&mut out, "{byte:02x}");
        }
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdjustmentPlan {
    pub plan_id: String,
    pub rationale: String,
    pub steps: Vec<AdjustmentStep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdjustmentStep {
    DropMemoryByRank { max_items: usize },
    CompressMemoryToTokens { token_budget: usize },
    RedactRuntimeFields { fields: Vec<String> },
    ClampPersonaVerbosity { level: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyReport {
    pub before_state_hash: String,
    pub after_state_hash: String,
    pub changed_components: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FitDecision {
    Fitted,
    Adjust(AdjustmentPlan),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReasoningContextError {
    ContextPolicyFitFailed(String),
    NonProgressDetected(String),
    ApplyFailed(String),
}

pub trait Composer {
    fn compose(&self, state: &ContextState) -> ComposePrompt;
}

pub trait PolicyFitter {
    fn fit(&self, prompt: &ComposePrompt, state: &ContextState) -> FitDecision;
}

pub trait AdjustmentApplier {
    fn apply(
        &mut self,
        state: &mut ContextState,
        plan: &AdjustmentPlan,
    ) -> Result<ApplyReport, ReasoningContextError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NoOpComposer;

impl Composer for NoOpComposer {
    fn compose(&self, state: &ContextState) -> ComposePrompt {
        ComposePrompt {
            prompt_text: state.goal.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AlwaysFitPolicyFitter;

impl PolicyFitter for AlwaysFitPolicyFitter {
    fn fit(&self, _prompt: &ComposePrompt, _state: &ContextState) -> FitDecision {
        FitDecision::Fitted
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyConfig {
    pub max_prompt_chars: usize,
    pub max_memory_lines: usize,
    pub forbidden_runtime_fields: Vec<String>,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            max_prompt_chars: 12_000,
            max_memory_lines: 32,
            forbidden_runtime_fields: vec![
                "api_key".to_string(),
                "token".to_string(),
                "secret".to_string(),
            ],
        }
    }
}

impl PolicyConfig {
    pub fn from_metadata(metadata: &BTreeMap<String, String>) -> Self {
        let mut cfg = Self::default();
        if let Some(raw) = metadata.get("policy.max_prompt_chars")
            && let Ok(v) = raw.parse::<usize>()
            && v > 0
        {
            cfg.max_prompt_chars = v;
        }
        if let Some(raw) = metadata.get("policy.max_memory_lines")
            && let Ok(v) = raw.parse::<usize>()
            && v > 0
        {
            cfg.max_memory_lines = v;
        }
        if let Some(raw) = metadata.get("policy.forbidden_runtime_fields") {
            let parsed: Vec<String> = raw
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToString::to_string)
                .collect();
            if !parsed.is_empty() {
                cfg.forbidden_runtime_fields = parsed;
            }
        }
        cfg
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeuristicPolicyFitter {
    config: PolicyConfig,
}

impl Default for HeuristicPolicyFitter {
    fn default() -> Self {
        Self {
            config: PolicyConfig::default(),
        }
    }
}

impl HeuristicPolicyFitter {
    pub fn new(config: PolicyConfig) -> Self {
        Self { config }
    }
}

impl PolicyFitter for HeuristicPolicyFitter {
    fn fit(&self, prompt: &ComposePrompt, state: &ContextState) -> FitDecision {
        let mut steps = Vec::new();

        let mut matches = Vec::new();
        for field in &self.config.forbidden_runtime_fields {
            if state.runtime.contains(field) {
                matches.push(field.clone());
            }
        }
        if !matches.is_empty() {
            steps.push(AdjustmentStep::RedactRuntimeFields { fields: matches });
        }

        let memory_line_count = state.memory.lines().filter(|v| !v.trim().is_empty()).count();
        if memory_line_count > self.config.max_memory_lines {
            steps.push(AdjustmentStep::DropMemoryByRank {
                max_items: self.config.max_memory_lines,
            });
        }

        if prompt.prompt_text.len() > self.config.max_prompt_chars {
            if !state.memory.is_empty() {
                let word_count = state.memory.split_whitespace().count();
                let next_budget = word_count.saturating_mul(3) / 4;
                steps.push(AdjustmentStep::CompressMemoryToTokens {
                    token_budget: next_budget,
                });
            }
            if !state.persona.contains("verbosity=low") {
                steps.push(AdjustmentStep::ClampPersonaVerbosity {
                    level: "low".to_string(),
                });
            }
        }

        if steps.is_empty() {
            FitDecision::Fitted
        } else {
            let step_tags: Vec<&str> = steps
                .iter()
                .map(|step| match step {
                    AdjustmentStep::DropMemoryByRank { .. } => "drop_memory",
                    AdjustmentStep::CompressMemoryToTokens { .. } => "compress_memory",
                    AdjustmentStep::RedactRuntimeFields { .. } => "redact_runtime",
                    AdjustmentStep::ClampPersonaVerbosity { .. } => "clamp_persona",
                })
                .collect();
            FitDecision::Adjust(AdjustmentPlan {
                plan_id: format!("policy-adjust-{}", step_tags.join("-")),
                rationale: "policy_fit_required".to_string(),
                steps,
            })
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DeterministicAdjustmentApplier;

impl AdjustmentApplier for DeterministicAdjustmentApplier {
    fn apply(
        &mut self,
        state: &mut ContextState,
        plan: &AdjustmentPlan,
    ) -> Result<ApplyReport, ReasoningContextError> {
        let before_state_hash = state.state_hash();
        let mut changed_components = Vec::new();

        for step in &plan.steps {
            match step {
                AdjustmentStep::DropMemoryByRank { max_items } => {
                    let items: Vec<&str> = state
                        .memory
                        .split('\n')
                        .filter(|v| !v.trim().is_empty())
                        .take(*max_items)
                        .collect();
                    let updated = items.join("\n");
                    if updated != state.memory {
                        state.memory = updated;
                        changed_components.push("memory".to_string());
                    }
                }
                AdjustmentStep::CompressMemoryToTokens { token_budget } => {
                    let words: Vec<&str> = state.memory.split_whitespace().take(*token_budget).collect();
                    let updated = words.join(" ");
                    if updated != state.memory {
                        state.memory = updated;
                        changed_components.push("memory".to_string());
                    }
                }
                AdjustmentStep::RedactRuntimeFields { fields } => {
                    let mut updated = state.runtime.clone();
                    for field in fields {
                        updated = updated.replace(field, "[REDACTED]");
                    }
                    if updated != state.runtime {
                        state.runtime = updated;
                        changed_components.push("runtime".to_string());
                    }
                }
                AdjustmentStep::ClampPersonaVerbosity { level } => {
                    let updated = format!("verbosity={}", level);
                    if updated != state.persona {
                        state.persona = updated;
                        changed_components.push("persona".to_string());
                    }
                }
            }
        }

        let after_state_hash = state.state_hash();
        Ok(ApplyReport {
            before_state_hash,
            after_state_hash,
            changed_components,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FitLoopOutcome {
    pub prompt: ComposePrompt,
    pub iterations: u64,
    pub records: Vec<FitLoopRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FitLoopRecord {
    pub iteration: u64,
    pub decision: String,
    pub plan_id: Option<String>,
    pub before_state_hash: Option<String>,
    pub after_state_hash: Option<String>,
}

pub fn run_fit_loop<C, F, A>(
    state: &mut ContextState,
    composer: &C,
    fitter: &F,
    applier: &mut A,
    max_iters: u64,
) -> Result<FitLoopOutcome, ReasoningContextError>
where
    C: Composer,
    F: PolicyFitter,
    A: AdjustmentApplier,
{
    if max_iters == 0 {
        return Err(ReasoningContextError::ContextPolicyFitFailed(
            "max_iters_must_be_positive".to_string(),
        ));
    }

    let mut seen_hashes = BTreeSet::new();
    let mut records = Vec::new();
    for iteration in 1..=max_iters {
        let prompt = composer.compose(state);
        match fitter.fit(&prompt, state) {
            FitDecision::Fitted => {
                records.push(FitLoopRecord {
                    iteration,
                    decision: "fitted".to_string(),
                    plan_id: None,
                    before_state_hash: None,
                    after_state_hash: None,
                });
                return Ok(FitLoopOutcome {
                    prompt,
                    iterations: iteration,
                    records,
                });
            }
            FitDecision::Adjust(plan) => {
                let report = applier.apply(state, &plan)?;
                records.push(FitLoopRecord {
                    iteration,
                    decision: "adjusted".to_string(),
                    plan_id: Some(plan.plan_id.clone()),
                    before_state_hash: Some(report.before_state_hash.clone()),
                    after_state_hash: Some(report.after_state_hash.clone()),
                });
                if report.before_state_hash == report.after_state_hash {
                    return Err(ReasoningContextError::NonProgressDetected(format!(
                        "no_state_change plan_id={}",
                        plan.plan_id
                    )));
                }
                if !seen_hashes.insert(report.after_state_hash) {
                    return Err(ReasoningContextError::NonProgressDetected(
                        "state_hash_repeated".to_string(),
                    ));
                }
            }
        }
    }

    Err(ReasoningContextError::ContextPolicyFitFailed(format!(
        "max_iters_exceeded max_iters={}",
        max_iters
    )))
}
