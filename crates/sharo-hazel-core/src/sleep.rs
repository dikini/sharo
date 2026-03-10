use crate::ingest::validate_bulk_submit_batches;
use crate::proposal::ProposalBatch;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SleepBudget {
    pub max_batches: usize,
    pub max_proposals_per_batch: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SleepJobOutput {
    pub run_id: String,
    pub batches: Vec<ProposalBatch>,
}

pub fn validate_sleep_budget(budget: &SleepBudget) -> Result<(), String> {
    if budget.max_batches == 0 {
        return Err("sleep_budget_invalid reason=max_batches_zero".to_string());
    }
    if budget.max_proposals_per_batch == 0 {
        return Err("sleep_budget_invalid reason=max_proposals_per_batch_zero".to_string());
    }
    Ok(())
}

pub fn validate_sleep_output(output: &SleepJobOutput, budget: &SleepBudget) -> Result<(), String> {
    if output.run_id.trim().is_empty() {
        return Err("sleep_output_invalid reason=run_id_required".to_string());
    }
    if output.batches.len() > budget.max_batches {
        return Err(format!(
            "sleep_output_invalid reason=batch_budget_exceeded actual={} max={}",
            output.batches.len(),
            budget.max_batches
        ));
    }
    for batch in &output.batches {
        if batch.proposals.len() > budget.max_proposals_per_batch {
            return Err(format!(
                "sleep_output_invalid reason=proposal_budget_exceeded batch_id={} actual={} max={}",
                batch.batch_id,
                batch.proposals.len(),
                budget.max_proposals_per_batch
            ));
        }
    }
    validate_bulk_submit_batches(&output.batches)
}
