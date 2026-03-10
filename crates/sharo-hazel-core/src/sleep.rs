use crate::ingest::validate_proposal_batches;
use crate::proposal::ProposalBatch;
use sha2::{Digest, Sha256};

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

pub fn derive_sleep_run_id(batches: &[ProposalBatch]) -> Result<String, String> {
    let mut rows: Vec<String> = batches
        .iter()
        .map(|batch| {
            serde_json::to_string(batch).map_err(|error| {
                format!("sleep_output_invalid reason=run_id_serialize_failed error={error}")
            })
        })
        .collect::<Result<_, _>>()?;
    rows.sort();
    let mut hasher = Sha256::new();
    for row in rows {
        hasher.update(row.as_bytes());
        hasher.update(b"\n");
    }
    let digest = hasher.finalize();
    Ok(format!("sleep-run-v2-{digest:x}"))
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
    let expected_run_id = derive_sleep_run_id(&output.batches)?;
    if output.run_id != expected_run_id {
        return Err(format!(
            "sleep_output_invalid reason=run_id_mismatch actual={} expected={}",
            output.run_id, expected_run_id
        ));
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
    validate_proposal_batches(&output.batches)
}
