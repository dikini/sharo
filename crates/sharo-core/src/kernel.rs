use crate::protocol::{ResolveApprovalResponse, SubmitTaskOpRequest, SubmitTaskOpResponse};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelSubmitInput {
    pub request: SubmitTaskOpRequest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelSubmitResult {
    pub response: SubmitTaskOpResponse,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelApprovalInput {
    pub approval_id: String,
    pub decision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelApprovalResult {
    pub response: ResolveApprovalResponse,
}

pub trait KernelPort {
    fn submit_task(&mut self, input: KernelSubmitInput) -> Result<KernelSubmitResult, String>;
    fn resolve_approval(&mut self, input: KernelApprovalInput) -> Result<KernelApprovalResult, String>;
}
