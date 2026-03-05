use std::collections::BTreeSet;

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
        format!(
            "system={}|persona={}|memory={}|runtime={}|goal={}",
            self.system, self.persona, self.memory, self.runtime, self.goal
        )
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
