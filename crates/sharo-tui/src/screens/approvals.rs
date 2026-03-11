use sharo_core::protocol::ApprovalSummary;

use crate::screens::sanitize_for_terminal;

pub fn render_approvals(approvals: &[ApprovalSummary]) -> String {
    let mut out = String::new();
    for approval in approvals {
        out.push_str(&format!(
            "{} [{}] {}\n",
            sanitize_for_terminal(&approval.approval_id),
            sanitize_for_terminal(&approval.state),
            sanitize_for_terminal(&approval.reason)
        ));
    }
    out
}
