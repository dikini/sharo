use sharo_core::protocol::{ApprovalSummary, SessionView, TaskSummary};

use crate::screens::sanitize_for_terminal;

pub fn render_chat_view(session: &SessionView) -> String {
    let mut out = format!(
        "session: {} ({})\n",
        sanitize_for_terminal(&session.session_label),
        sanitize_for_terminal(&session.session_id)
    );
    for task in &session.tasks {
        out.push_str(&format_task(task));
    }
    if let Some(approval) = active_approval(session) {
        out.push_str(&format_inline_approval(approval));
    }
    out
}

pub fn format_inline_approval(approval: &ApprovalSummary) -> String {
    format!(
        "approval required: {} task={} reason={}\n",
        sanitize_for_terminal(&approval.approval_id),
        sanitize_for_terminal(&approval.task_id),
        sanitize_for_terminal(&approval.reason)
    )
}

fn format_task(task: &TaskSummary) -> String {
    let preview = task
        .result_preview
        .as_deref()
        .unwrap_or(task.current_step_summary.as_str());
    format!(
        "task {} [{}] {}\n",
        sanitize_for_terminal(&task.task_id),
        sanitize_for_terminal(&task.task_state),
        sanitize_for_terminal(preview)
    )
}

fn active_approval(session: &SessionView) -> Option<&ApprovalSummary> {
    session
        .active_blocking_task_id
        .as_ref()
        .and_then(|task_id| {
            session
                .pending_approvals
                .iter()
                .find(|approval| approval.task_id == *task_id)
        })
}

#[cfg(test)]
mod tests {
    use sharo_core::protocol::{ApprovalSummary, SessionView, TaskSummary};

    use super::render_chat_view;

    #[test]
    fn chat_view_renders_inline_approval_block_for_active_turn() {
        let rendered = render_chat_view(&SessionView {
            session_id: "session-1".to_string(),
            session_label: "alpha".to_string(),
            tasks: vec![TaskSummary {
                task_id: "task-1".to_string(),
                session_id: "session-1".to_string(),
                task_state: "awaiting_approval".to_string(),
                current_step_summary: "awaiting approval".to_string(),
                blocking_reason: Some("approval_required".to_string()),
                coordination_summary: None,
                result_preview: None,
            }],
            pending_approvals: vec![ApprovalSummary {
                approval_id: "approval-1".to_string(),
                task_id: "task-1".to_string(),
                state: "pending".to_string(),
                reason: "policy require_approval".to_string(),
            }],
            latest_result_preview: None,
            active_blocking_task_id: Some("task-1".to_string()),
        });

        assert!(rendered.contains("approval required: approval-1"));
    }

    #[test]
    fn chat_view_escapes_control_sequences_from_daemon_text() {
        let rendered = render_chat_view(&SessionView {
            session_id: "session-\u{1b}[31m1".to_string(),
            session_label: "alpha\nbeta".to_string(),
            tasks: vec![TaskSummary {
                task_id: "task-\u{1b}[2J".to_string(),
                session_id: "session-1".to_string(),
                task_state: "completed".to_string(),
                current_step_summary: "step".to_string(),
                blocking_reason: None,
                coordination_summary: None,
                result_preview: Some("line1\nline2\u{1b}[0m".to_string()),
            }],
            pending_approvals: Vec::new(),
            latest_result_preview: None,
            active_blocking_task_id: None,
        });

        assert!(!rendered.contains('\u{1b}'));
        assert!(rendered.contains("alpha\\nbeta"));
        assert!(rendered.contains("line1\\nline2"));
    }
}
