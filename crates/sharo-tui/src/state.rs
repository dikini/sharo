use sharo_core::protocol::{ApprovalSummary, SessionSummary, SessionView};

use clap::ValueEnum;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Screen {
    Chat,
    Hazel,
    Sessions,
    Approvals,
    TraceArtifacts,
    Settings,
}

impl Screen {
    pub fn title(self) -> &'static str {
        match self {
            Self::Chat => "Chat",
            Self::Hazel => "Hazel",
            Self::Sessions => "Sessions",
            Self::Approvals => "Approvals",
            Self::TraceArtifacts => "Trace/Artifacts",
            Self::Settings => "Settings",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppState {
    active_screen: Screen,
    active_session_id: Option<String>,
    daemon_connected: bool,
    daemon_warning: Option<String>,
    sessions: Vec<SessionSummary>,
    current_session_view: Option<SessionView>,
    approvals: Vec<ApprovalSummary>,
    hazel_panel: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            active_screen: Screen::Chat,
            active_session_id: None,
            daemon_connected: false,
            daemon_warning: None,
            sessions: Vec::new(),
            current_session_view: None,
            approvals: Vec::new(),
            hazel_panel: "hazel: no data\n".to_string(),
        }
    }
}

impl AppState {
    pub fn active_screen(&self) -> Screen {
        self.active_screen
    }

    pub fn active_session_id(&self) -> Option<&str> {
        self.active_session_id.as_deref()
    }

    pub fn daemon_connected(&self) -> bool {
        self.daemon_connected
    }

    pub fn daemon_warning(&self) -> Option<&str> {
        self.daemon_warning.as_deref()
    }

    pub fn sessions(&self) -> &[SessionSummary] {
        &self.sessions
    }

    pub fn current_session_view(&self) -> Option<&SessionView> {
        self.current_session_view.as_ref()
    }

    pub fn approvals(&self) -> &[ApprovalSummary] {
        &self.approvals
    }

    pub fn hazel_panel(&self) -> &str {
        &self.hazel_panel
    }

    #[allow(dead_code)]
    pub fn set_active_screen(&mut self, screen: Screen) {
        self.active_screen = screen;
    }

    #[allow(dead_code)]
    pub fn set_active_session_id(&mut self, session_id: Option<String>) {
        self.active_session_id = session_id;
    }

    pub fn set_daemon_connected(&mut self, connected: bool) {
        self.daemon_connected = connected;
    }

    pub fn set_daemon_warning(&mut self, warning: Option<String>) {
        self.daemon_warning = warning;
    }

    pub fn set_sessions(&mut self, sessions: Vec<SessionSummary>) {
        self.sessions = sessions;
    }

    pub fn set_current_session_view(&mut self, session: Option<SessionView>) {
        self.current_session_view = session;
    }

    pub fn set_approvals(&mut self, approvals: Vec<ApprovalSummary>) {
        self.approvals = approvals;
    }

    pub fn set_hazel_panel(&mut self, hazel_panel: String) {
        self.hazel_panel = hazel_panel;
    }
}

#[cfg(test)]
mod tests {
    use super::{AppState, Screen};

    #[test]
    fn default_screen_is_chat() {
        let state = AppState::default();
        assert_eq!(state.active_screen(), Screen::Chat);
    }

    #[test]
    fn active_session_state_is_distinct_from_screen_focus_state() {
        let mut state = AppState::default();
        state.set_active_session_id(Some("session-42".to_string()));
        state.set_active_screen(Screen::Settings);

        assert_eq!(state.active_screen(), Screen::Settings);
        assert_eq!(state.active_session_id(), Some("session-42"));
    }
}
