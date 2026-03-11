use sharo_core::protocol::SessionSummary;

use crate::screens::sanitize_for_terminal;

pub fn render_sessions(sessions: &[SessionSummary], active_session_id: Option<&str>) -> String {
    let mut out = String::new();
    for session in sessions {
        let marker = if active_session_id == Some(session.session_id.as_str()) {
            "*"
        } else {
            " "
        };
        out.push_str(&format!(
            "{} {} [{}] {}\n",
            marker,
            sanitize_for_terminal(&session.session_label),
            sanitize_for_terminal(&session.session_status),
            sanitize_for_terminal(&session.session_id)
        ));
    }
    out
}
