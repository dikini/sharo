use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

use sharo_core::mcp::{McpRuntimeStatus, McpServerSummary};
use sharo_core::protocol::{
    ArtifactSummary, DaemonRequest, DaemonResponse, GetArtifactsRequest, GetArtifactsResponse,
    GetRuntimeStatusResponse, GetSessionViewRequest, GetSessionViewResponse, GetTraceRequest,
    GetTraceResponse, ListMcpServersResponse, ListPendingApprovalsResponse, ListSessionsResponse,
    ListSkillsRequest, ListSkillsResponse, RegisterSessionRequest, RegisterSessionResponse,
    ResolveApprovalRequest, ResolveApprovalResponse, SessionSummary, SetSessionSkillsRequest,
    SetSessionSkillsResponse, SubmitTaskOpRequest, SubmitTaskOpResponse, TaskSummary, TraceSummary,
    UpdateMcpServerStateRequest, UpdateMcpServerStateResponse,
};
use sharo_core::skills::SkillCatalogEntry;

use crate::commands::{SlashCommand, parse_slash_command};
use crate::screens::sanitize_for_terminal;
use crate::screens::{approvals, artifacts, chat, sessions, settings};
use crate::state::{AppState, Screen};

const MAX_RESPONSE_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone)]
pub struct DaemonClient {
    socket_path: PathBuf,
}

impl DaemonClient {
    pub fn new(socket_path: impl AsRef<Path>) -> Self {
        Self {
            socket_path: socket_path.as_ref().to_path_buf(),
        }
    }

    #[allow(dead_code)]
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    pub fn get_runtime_status(&self) -> Result<GetRuntimeStatusResponse, String> {
        let response = self.send(&DaemonRequest::GetRuntimeStatus)?;
        match response {
            DaemonResponse::GetRuntimeStatus(status) => Ok(status),
            DaemonResponse::Error { message } => Err(message),
            other => Err(format!("unexpected_daemon_response response={other:?}")),
        }
    }

    pub fn list_sessions(&self) -> Result<ListSessionsResponse, String> {
        let response = self.send(&DaemonRequest::ListSessions)?;
        match response {
            DaemonResponse::ListSessions(sessions) => Ok(sessions),
            DaemonResponse::Error { message } => Err(message),
            other => Err(format!("unexpected_daemon_response response={other:?}")),
        }
    }

    pub fn register_session(&self, session_label: &str) -> Result<RegisterSessionResponse, String> {
        let response = self.send(&DaemonRequest::RegisterSession(RegisterSessionRequest {
            session_label: session_label.to_string(),
        }))?;
        match response {
            DaemonResponse::RegisterSession(session) => Ok(session),
            DaemonResponse::Error { message } => Err(message),
            other => Err(format!("unexpected_daemon_response response={other:?}")),
        }
    }

    pub fn get_session_view(&self, session_id: &str) -> Result<GetSessionViewResponse, String> {
        let response = self.send(&DaemonRequest::GetSessionView(GetSessionViewRequest {
            session_id: session_id.to_string(),
            task_limit: Some(32),
        }))?;
        match response {
            DaemonResponse::GetSessionView(view) => Ok(view),
            DaemonResponse::Error { message } => Err(message),
            other => Err(format!("unexpected_daemon_response response={other:?}")),
        }
    }

    pub fn submit_turn(
        &self,
        session_id: &str,
        goal: &str,
    ) -> Result<SubmitTaskOpResponse, String> {
        let response = self.send(&DaemonRequest::SubmitTask(SubmitTaskOpRequest {
            session_id: Some(session_id.to_string()),
            goal: goal.to_string(),
            idempotency_key: None,
        }))?;
        match response {
            DaemonResponse::SubmitTask(submit) => Ok(submit),
            DaemonResponse::Error { message } => Err(message),
            other => Err(format!("unexpected_daemon_response response={other:?}")),
        }
    }

    pub fn list_pending_approvals(&self) -> Result<ListPendingApprovalsResponse, String> {
        let response = self.send(&DaemonRequest::ListPendingApprovals)?;
        match response {
            DaemonResponse::ListPendingApprovals(approvals) => Ok(approvals),
            DaemonResponse::Error { message } => Err(message),
            other => Err(format!("unexpected_daemon_response response={other:?}")),
        }
    }

    pub fn resolve_approval(
        &self,
        approval_id: &str,
        decision: &str,
    ) -> Result<ResolveApprovalResponse, String> {
        let response = self.send(&DaemonRequest::ResolveApproval(ResolveApprovalRequest {
            approval_id: approval_id.to_string(),
            decision: decision.to_string(),
        }))?;
        match response {
            DaemonResponse::ResolveApproval(result) => Ok(result),
            DaemonResponse::Error { message } => Err(message),
            other => Err(format!("unexpected_daemon_response response={other:?}")),
        }
    }

    pub fn list_skills(&self, session_id: Option<&str>) -> Result<ListSkillsResponse, String> {
        let response = self.send(&DaemonRequest::ListSkills(ListSkillsRequest {
            session_id: session_id.map(ToOwned::to_owned),
        }))?;
        match response {
            DaemonResponse::ListSkills(skills) => Ok(skills),
            DaemonResponse::Error { message } => Err(message),
            other => Err(format!("unexpected_daemon_response response={other:?}")),
        }
    }

    pub fn set_session_skills(
        &self,
        session_id: &str,
        active_skill_ids: Vec<String>,
    ) -> Result<SetSessionSkillsResponse, String> {
        let response = self.send(&DaemonRequest::SetSessionSkills(SetSessionSkillsRequest {
            session_id: session_id.to_string(),
            active_skill_ids,
        }))?;
        match response {
            DaemonResponse::SetSessionSkills(skills) => Ok(skills),
            DaemonResponse::Error { message } => Err(message),
            other => Err(format!("unexpected_daemon_response response={other:?}")),
        }
    }

    pub fn list_mcp_servers(&self) -> Result<ListMcpServersResponse, String> {
        let response = self.send(&DaemonRequest::ListMcpServers)?;
        match response {
            DaemonResponse::ListMcpServers(servers) => Ok(servers),
            DaemonResponse::Error { message } => Err(message),
            other => Err(format!("unexpected_daemon_response response={other:?}")),
        }
    }

    pub fn update_mcp_server_state(
        &self,
        server_id: &str,
        enabled: bool,
    ) -> Result<UpdateMcpServerStateResponse, String> {
        let response = self.send(&DaemonRequest::UpdateMcpServerState(
            UpdateMcpServerStateRequest {
                server_id: server_id.to_string(),
                enabled,
            },
        ))?;
        match response {
            DaemonResponse::UpdateMcpServerState(server) => Ok(server),
            DaemonResponse::Error { message } => Err(message),
            other => Err(format!("unexpected_daemon_response response={other:?}")),
        }
    }

    pub fn get_trace(&self, task_id: &str) -> Result<GetTraceResponse, String> {
        let response = self.send(&DaemonRequest::GetTrace(GetTraceRequest {
            task_id: task_id.to_string(),
        }))?;
        match response {
            DaemonResponse::GetTrace(trace) => Ok(trace),
            DaemonResponse::Error { message } => Err(message),
            other => Err(format!("unexpected_daemon_response response={other:?}")),
        }
    }

    pub fn get_artifacts(&self, task_id: &str) -> Result<GetArtifactsResponse, String> {
        let response = self.send(&DaemonRequest::GetArtifacts(GetArtifactsRequest {
            task_id: task_id.to_string(),
        }))?;
        match response {
            DaemonResponse::GetArtifacts(artifacts) => Ok(artifacts),
            DaemonResponse::Error { message } => Err(message),
            other => Err(format!("unexpected_daemon_response response={other:?}")),
        }
    }

    fn send(&self, request: &DaemonRequest) -> Result<DaemonResponse, String> {
        let mut stream = UnixStream::connect(&self.socket_path).map_err(|error| {
            format!(
                "daemon_connect_failed path={} error={error}",
                self.socket_path.display()
            )
        })?;
        let payload = serde_json::to_string(request)
            .map_err(|error| format!("daemon_request_serialize_failed error={error}"))?;
        writeln!(stream, "{payload}")
            .map_err(|error| format!("daemon_request_write_failed error={error}"))?;
        let line = read_response_line(stream)?;
        serde_json::from_str(line.trim())
            .map_err(|error| format!("daemon_response_parse_failed error={error}"))
    }
}

pub struct App {
    client: DaemonClient,
    state: AppState,
    runtime_status: Option<GetRuntimeStatusResponse>,
    active_skills: Vec<SkillCatalogEntry>,
    mcp_servers: Vec<McpServerSummary>,
    selected_task_id: Option<String>,
    selected_trace: Option<TraceSummary>,
    selected_artifacts: Vec<ArtifactSummary>,
}

struct SessionPresentationData {
    session: Option<sharo_core::protocol::SessionView>,
    active_skills: Vec<SkillCatalogEntry>,
    selected_task_id: Option<String>,
    selected_trace: Option<TraceSummary>,
    selected_artifacts: Vec<ArtifactSummary>,
}

struct SettingsData {
    runtime_status: GetRuntimeStatusResponse,
    active_skills: Vec<SkillCatalogEntry>,
    mcp_servers: Vec<McpServerSummary>,
}

impl App {
    pub fn new(client: DaemonClient) -> Self {
        Self {
            client,
            state: AppState::default(),
            runtime_status: None,
            active_skills: Vec::new(),
            mcp_servers: Vec::new(),
            selected_task_id: None,
            selected_trace: None,
            selected_artifacts: Vec::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), String> {
        self.refresh_runtime_status()?;
        self.refresh_sessions()?;
        self.refresh_approvals()?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn state(&self) -> &AppState {
        &self.state
    }

    #[allow(dead_code)]
    pub fn state_mut(&mut self) -> &mut AppState {
        &mut self.state
    }

    pub fn render_shell(&self) -> String {
        let daemon_status = if self.state.daemon_connected() {
            "connected"
        } else {
            "disconnected"
        };
        let warning = self
            .state
            .daemon_warning()
            .map(|warning| format!("\nwarning: {}", sanitize_for_terminal(warning)))
            .unwrap_or_default();

        format!(
            "Sharo TUI\nscreen: {}\nscreens: {} | {} | {} | {} | {}\nactive session: {}\ndaemon: {}{}\n{}\n",
            self.state.active_screen().title(),
            Screen::Chat.title(),
            Screen::Sessions.title(),
            Screen::Approvals.title(),
            Screen::TraceArtifacts.title(),
            Screen::Settings.title(),
            sanitize_for_terminal(self.state.active_session_id().unwrap_or("none")),
            daemon_status,
            warning,
            self.render_active_screen(),
        )
    }

    pub fn refresh_sessions(&mut self) -> Result<(), String> {
        let sessions = self.client.list_sessions()?.sessions;
        let next_active_session_id = self
            .state
            .active_session_id()
            .map(ToOwned::to_owned)
            .or_else(|| sessions.first().map(|session| session.session_id.clone()));
        let presentation = self.load_session_presentation(next_active_session_id.as_deref())?;
        self.state.set_sessions(sessions);
        self.apply_session_presentation(next_active_session_id, presentation);
        Ok(())
    }

    pub fn create_session(&mut self, session_label: &str) -> Result<String, String> {
        let session = self.client.register_session(session_label)?;
        let session_id = session.session_id;
        let presentation = self.load_session_presentation(Some(&session_id))?;
        let sessions = self
            .client
            .list_sessions()
            .map(|response| response.sessions)
            .unwrap_or_else(|_| synthesize_created_session(self.state.sessions(), &session_id, session_label));
        self.state.set_sessions(sessions);
        self.apply_session_presentation(Some(session_id.clone()), presentation);
        Ok(session_id)
    }

    pub fn switch_session(&mut self, session_id: &str) -> Result<(), String> {
        let exists = self
            .state
            .sessions()
            .iter()
            .any(|session| session.session_id == session_id);
        if !exists {
            return Err(format!("session_not_found session_id={session_id}"));
        }
        let presentation = self.load_session_presentation(Some(session_id))?;
        self.apply_session_presentation(Some(session_id.to_string()), presentation);
        Ok(())
    }

    pub fn submit_turn(&mut self, goal: &str) -> Result<String, String> {
        let session_id = self.ensure_active_session()?;
        let response = self.client.submit_turn(&session_id, goal)?;
        self.refresh_sessions()?;
        self.refresh_approvals()?;
        self.refresh_current_session_view()?;
        Ok(response.task_id)
    }

    pub fn handle_chat_input(&mut self, input: &str) -> Result<String, String> {
        match parse_slash_command(input) {
            Ok(Some(command)) => self
                .dispatch_slash_command(command)
                .map_err(|error| sanitize_for_terminal(&error)),
            Ok(None) => {
                let task_id = self
                    .submit_turn(input)
                    .map_err(|error| sanitize_for_terminal(&error))?;
                Ok(format!(
                    "submitted task={}",
                    sanitize_for_terminal(&task_id)
                ))
            }
            Err(error) => Err(format!(
                "{} {}",
                error.code,
                sanitize_for_terminal(&error.message)
            )),
        }
    }

    pub fn resolve_approval(&mut self, approval_id: &str, decision: &str) -> Result<(), String> {
        let _ = self.client.resolve_approval(approval_id, decision)?;
        self.refresh_sessions()?;
        self.refresh_approvals()?;
        self.refresh_current_session_view()
    }

    pub fn render_chat(&self) -> String {
        self.state
            .current_session_view()
            .map(chat::render_chat_view)
            .unwrap_or_else(|| "no active session\n".to_string())
    }

    pub fn render_settings(&self) -> String {
        let runtime = self.runtime_status.as_ref();
        let model_profile_id = runtime.and_then(|status| status.status.model_profile_id.as_deref());
        let warnings = runtime
            .map(|status| status.status.warnings.as_slice())
            .unwrap_or(&[]);
        settings::render_settings(
            model_profile_id,
            warnings,
            &self.active_skills,
            &self.mcp_servers,
        )
    }

    pub fn render_trace_artifacts(&self) -> String {
        artifacts::render_trace_artifacts(
            self.selected_task_id.as_deref(),
            self.selected_trace.as_ref(),
            &self.selected_artifacts,
        )
    }

    fn refresh_approvals(&mut self) -> Result<(), String> {
        let approvals = self.client.list_pending_approvals()?.approvals;
        self.state.set_approvals(approvals);
        Ok(())
    }

    fn refresh_runtime_status(&mut self) -> Result<(), String> {
        let settings = self.load_settings_data(self.state.active_session_id())?;
        self.apply_settings_data(settings);
        Ok(())
    }

    fn refresh_settings_data(&mut self) -> Result<(), String> {
        let settings = self.load_settings_data(self.state.active_session_id())?;
        self.apply_settings_data(settings);
        Ok(())
    }

    fn refresh_current_session_view(&mut self) -> Result<(), String> {
        let active_session_id = self.state.active_session_id().map(ToOwned::to_owned);
        let presentation = self.load_session_presentation(active_session_id.as_deref())?;
        self.apply_session_presentation(active_session_id, presentation);
        Ok(())
    }

    fn load_session_presentation(
        &self,
        session_id: Option<&str>,
    ) -> Result<SessionPresentationData, String> {
        let session = match session_id {
            Some(session_id) => Some(self.client.get_session_view(session_id)?.session),
            None => None,
        };
        let active_skills = match session_id {
            Some(session_id) => self.client.list_skills(Some(session_id))?.skills,
            None => self.client.list_skills(None)?.skills,
        };
        let selected_task = session.as_ref().and_then(select_inspection_task);
        let (selected_task_id, selected_trace, selected_artifacts) = match selected_task {
            Some(task) => (
                Some(task.task_id.clone()),
                Some(self.client.get_trace(&task.task_id)?.trace),
                self.client.get_artifacts(&task.task_id)?.artifacts,
            ),
            None => (None, None, Vec::new()),
        };

        Ok(SessionPresentationData {
            session,
            active_skills,
            selected_task_id,
            selected_trace,
            selected_artifacts,
        })
    }

    fn load_settings_data(&self, session_id: Option<&str>) -> Result<SettingsData, String> {
        let runtime_status = self.client.get_runtime_status()?;
        let mcp_servers = self.client.list_mcp_servers()?.servers;
        let active_skills = match session_id {
            Some(session_id) => self.client.list_skills(Some(session_id))?.skills,
            None => self.client.list_skills(None)?.skills,
        };

        Ok(SettingsData {
            runtime_status,
            active_skills,
            mcp_servers,
        })
    }

    fn apply_session_presentation(
        &mut self,
        active_session_id: Option<String>,
        presentation: SessionPresentationData,
    ) {
        self.state.set_active_session_id(active_session_id);
        self.state.set_current_session_view(presentation.session);
        self.active_skills = presentation.active_skills;
        self.selected_task_id = presentation.selected_task_id;
        self.selected_trace = presentation.selected_trace;
        self.selected_artifacts = presentation.selected_artifacts;
    }

    fn apply_settings_data(&mut self, settings: SettingsData) {
        self.state
            .set_daemon_connected(settings.runtime_status.status.daemon_ready);
        self.state
            .set_daemon_warning(settings.runtime_status.status.warnings.first().cloned());
        self.runtime_status = Some(settings.runtime_status);
        self.active_skills = settings.active_skills;
        self.mcp_servers = settings.mcp_servers;
    }

    fn ensure_active_session(&mut self) -> Result<String, String> {
        if let Some(session_id) = self.state.active_session_id() {
            return Ok(session_id.to_string());
        }
        self.create_session("chat")
            .map(|session_id| session_id.to_string())
    }

    fn dispatch_slash_command(&mut self, command: SlashCommand) -> Result<String, String> {
        match command {
            SlashCommand::Sessions => {
                self.refresh_sessions()?;
                Ok(render_sessions_listing(self.state.sessions()))
            }
            SlashCommand::SessionNew { label } => {
                let label = label.unwrap_or_else(|| "chat".to_string());
                let session_id = self.create_session(&label)?;
                Ok(format!(
                    "session created: {} ({})",
                    sanitize_for_terminal(&label),
                    sanitize_for_terminal(&session_id)
                ))
            }
            SlashCommand::SessionSwitch { session_id } => {
                self.switch_session(&session_id)?;
                Ok(format!(
                    "active session: {}",
                    sanitize_for_terminal(&session_id)
                ))
            }
            SlashCommand::Approve { approval_id } => {
                self.resolve_approval(&approval_id, "approve")?;
                Ok(format!(
                    "approval resolved: {} [approved]",
                    sanitize_for_terminal(&approval_id)
                ))
            }
            SlashCommand::Deny { approval_id } => {
                self.resolve_approval(&approval_id, "deny")?;
                Ok(format!(
                    "approval resolved: {} [denied]",
                    sanitize_for_terminal(&approval_id)
                ))
            }
            SlashCommand::Skills => {
                let skills = self
                    .client
                    .list_skills(self.state.active_session_id())?
                    .skills;
                self.active_skills = skills.clone();
                Ok(render_skills_listing(&skills))
            }
            SlashCommand::SkillEnable { skill_id } => self.update_skill_activation(&skill_id, true),
            SlashCommand::SkillDisable { skill_id } => {
                self.update_skill_activation(&skill_id, false)
            }
            SlashCommand::Mcp => {
                let servers = self.client.list_mcp_servers()?.servers;
                self.mcp_servers = servers.clone();
                Ok(render_mcp_listing(&servers))
            }
            SlashCommand::McpEnable { server_id } => {
                let response = self.client.update_mcp_server_state(&server_id, true)?;
                self.refresh_runtime_status()?;
                Ok(format!(
                    "mcp server: {} [enabled={}]",
                    sanitize_for_terminal(&response.server.server_id),
                    response.server.enabled
                ))
            }
            SlashCommand::McpDisable { server_id } => {
                let response = self.client.update_mcp_server_state(&server_id, false)?;
                self.refresh_runtime_status()?;
                Ok(format!(
                    "mcp server: {} [enabled={}]",
                    sanitize_for_terminal(&response.server.server_id),
                    response.server.enabled
                ))
            }
            SlashCommand::Model => {
                let runtime = self.client.get_runtime_status()?;
                Ok(format!(
                    "model profile: {}",
                    sanitize_for_terminal(
                        runtime.status.model_profile_id.as_deref().unwrap_or("none")
                    )
                ))
            }
        }
    }

    fn update_skill_activation(&mut self, skill_id: &str, enable: bool) -> Result<String, String> {
        let session_id = self.ensure_active_session()?;
        let current = self.client.list_skills(Some(&session_id))?.skills;
        let mut active_skill_ids = current
            .iter()
            .filter(|skill| skill.is_active)
            .map(|skill| skill.skill_id.clone())
            .collect::<Vec<_>>();
        let exists = current.iter().any(|skill| skill.skill_id == skill_id);
        if !exists {
            return Err(format!(
                "skill_not_found skill_id={}",
                sanitize_for_terminal(skill_id)
            ));
        }
        if enable {
            if !active_skill_ids.iter().any(|active| active == skill_id) {
                active_skill_ids.push(skill_id.to_string());
            }
        } else {
            active_skill_ids.retain(|active| active != skill_id);
        }
        active_skill_ids.sort();
        active_skill_ids.dedup();
        let response = self
            .client
            .set_session_skills(&session_id, active_skill_ids)?;
        self.refresh_settings_data()?;
        Ok(format!(
            "skill {}: {}",
            if enable { "enabled" } else { "disabled" },
            sanitize_for_terminal(
                response
                    .active_skill_ids
                    .iter()
                    .find(|active| active.as_str() == skill_id)
                    .map_or(skill_id, |active| active.as_str())
            )
        ))
    }

    fn render_active_screen(&self) -> String {
        match self.state.active_screen() {
            Screen::Chat => self.render_chat(),
            Screen::Sessions => {
                sessions::render_sessions(self.state.sessions(), self.state.active_session_id())
            }
            Screen::Approvals => approvals::render_approvals(self.state.approvals()),
            Screen::TraceArtifacts => self.render_trace_artifacts(),
            Screen::Settings => self.render_settings(),
        }
    }
}

fn select_inspection_task(session: &sharo_core::protocol::SessionView) -> Option<&TaskSummary> {
    session.tasks.last()
}

fn render_sessions_listing(sessions: &[sharo_core::protocol::SessionSummary]) -> String {
    if sessions.is_empty() {
        return "sessions: none".to_string();
    }
    let lines = sessions
        .iter()
        .map(|session| {
            format!(
                "{} [{}]",
                sanitize_for_terminal(&session.session_id),
                sanitize_for_terminal(&session.session_status)
            )
        })
        .collect::<Vec<_>>();
    format!("sessions:\n{}", lines.join("\n"))
}

fn synthesize_created_session(
    existing_sessions: &[SessionSummary],
    session_id: &str,
    session_label: &str,
) -> Vec<SessionSummary> {
    let next_activity_sequence = existing_sessions
        .iter()
        .map(|session| session.activity_sequence)
        .max()
        .unwrap_or(0)
        + 1;
    let mut sessions = existing_sessions
        .iter()
        .filter(|session| session.session_id != session_id)
        .cloned()
        .collect::<Vec<_>>();
    sessions.push(SessionSummary {
        session_id: session_id.to_string(),
        session_label: session_label.to_string(),
        session_status: "idle".to_string(),
        activity_sequence: next_activity_sequence,
        latest_task_id: None,
        latest_task_state: None,
        latest_result_preview: None,
        has_pending_approval: false,
    });
    sessions.sort_by_key(|session| std::cmp::Reverse(session.activity_sequence));
    sessions
}

fn render_skills_listing(skills: &[sharo_core::skills::SkillCatalogEntry]) -> String {
    if skills.is_empty() {
        return "skills: none".to_string();
    }
    let lines = skills
        .iter()
        .map(|skill| {
            format!(
                "{} [{}]",
                sanitize_for_terminal(&skill.skill_id),
                if skill.is_active {
                    "active"
                } else {
                    "inactive"
                }
            )
        })
        .collect::<Vec<_>>();
    format!("skills:\n{}", lines.join("\n"))
}

fn render_mcp_listing(servers: &[sharo_core::mcp::McpServerSummary]) -> String {
    if servers.is_empty() {
        return "mcp servers: none".to_string();
    }
    let lines = servers
        .iter()
        .map(|server| {
            format!(
                "{} [enabled={} status={}]",
                sanitize_for_terminal(&server.server_id),
                server.enabled,
                mcp_runtime_status_label(server.runtime_status)
            )
        })
        .collect::<Vec<_>>();
    format!("mcp servers:\n{}", lines.join("\n"))
}

fn mcp_runtime_status_label(status: McpRuntimeStatus) -> &'static str {
    match status {
        McpRuntimeStatus::Disabled => "disabled",
        McpRuntimeStatus::Configured => "configured",
    }
}

fn read_response_line(mut stream: UnixStream) -> Result<String, String> {
    let mut bytes = Vec::new();
    let mut buf = [0_u8; 1];
    loop {
        let read = stream
            .read(&mut buf)
            .map_err(|error| format!("daemon_response_read_failed error={error}"))?;
        if read == 0 {
            break;
        }
        if buf[0] == b'\n' {
            break;
        }
        bytes.push(buf[0]);
        if bytes.len() > MAX_RESPONSE_BYTES {
            return Err(format!(
                "daemon_response_too_large max_bytes={} actual_bytes>{}",
                MAX_RESPONSE_BYTES, MAX_RESPONSE_BYTES
            ));
        }
    }
    String::from_utf8(bytes).map_err(|error| format!("daemon_response_utf8_invalid error={error}"))
}

#[cfg(test)]
mod tests {
    use sharo_core::protocol::SessionSummary;

    use super::synthesize_created_session;

    #[test]
    fn create_session_fallback_synthesizes_new_idle_session_locally() {
        let sessions = synthesize_created_session(
            &[SessionSummary {
                session_id: "session-1".to_string(),
                session_label: "alpha".to_string(),
                session_status: "completed".to_string(),
                activity_sequence: 3,
                latest_task_id: Some("task-1".to_string()),
                latest_task_state: Some("completed".to_string()),
                latest_result_preview: Some("done".to_string()),
                has_pending_approval: false,
            }],
            "session-2",
            "beta",
        );

        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].session_id, "session-2");
        assert_eq!(sessions[0].session_label, "beta");
        assert_eq!(sessions[0].session_status, "idle");
        assert!(sessions[0].activity_sequence > sessions[1].activity_sequence);
    }
}
