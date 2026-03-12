use std::collections::BTreeSet;
use std::io::{Stdout, stdout};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

use crossterm::cursor::Show;
use crossterm::event::{
    self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::app::App;
use crate::layout::render_frame;
use crate::screens::sanitize_for_terminal;
use crate::state::Screen;

const TICK_INTERVAL: Duration = Duration::from_millis(750);
const PENDING_WORK_POLL_INTERVAL: Duration = Duration::from_millis(25);

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ComposerState {
    text: String,
    cursor_chars: usize,
}

impl ComposerState {
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub fn cursor_chars(&self) -> usize {
        self.cursor_chars
    }

    fn len_chars(&self) -> usize {
        self.text.chars().count()
    }

    fn cursor_byte_index(&self) -> usize {
        char_to_byte_index(&self.text, self.cursor_chars)
    }

    fn insert_char(&mut self, ch: char) {
        let byte_index = self.cursor_byte_index();
        self.text.insert(byte_index, ch);
        self.cursor_chars += 1;
    }

    fn insert_newline(&mut self) {
        self.insert_char('\n');
    }

    fn move_left(&mut self) {
        if self.cursor_chars > 0 {
            self.cursor_chars -= 1;
        }
    }

    fn move_right(&mut self) {
        if self.cursor_chars < self.len_chars() {
            self.cursor_chars += 1;
        }
    }

    fn backspace(&mut self) {
        if self.cursor_chars == 0 {
            return;
        }
        let end = self.cursor_byte_index();
        let start = char_to_byte_index(&self.text, self.cursor_chars - 1);
        self.text.drain(start..end);
        self.cursor_chars -= 1;
    }

    fn delete_forward(&mut self) {
        if self.cursor_chars >= self.len_chars() {
            return;
        }
        let start = self.cursor_byte_index();
        let end = char_to_byte_index(&self.text, self.cursor_chars + 1);
        self.text.drain(start..end);
    }

    fn move_home(&mut self) {
        let (line_start, _) = self.line_bounds();
        self.cursor_chars = line_start;
    }

    fn move_end(&mut self) {
        let (_, line_end) = self.line_bounds();
        self.cursor_chars = line_end;
    }

    fn move_up(&mut self) {
        let current_column = self.cursor_column();
        let (line_start, _) = self.line_bounds();
        if line_start == 0 {
            return;
        }
        let previous_line_end = line_start - 1;
        let previous_line_start = self.text[..char_to_byte_index(&self.text, previous_line_end)]
            .chars()
            .rev()
            .position(|ch| ch == '\n')
            .map(|offset| previous_line_end - offset)
            .unwrap_or(0);
        let previous_line_len = previous_line_end - previous_line_start;
        self.cursor_chars = previous_line_start + current_column.min(previous_line_len);
    }

    fn move_down(&mut self) {
        let current_column = self.cursor_column();
        let (_, line_end) = self.line_bounds();
        if line_end == self.len_chars() {
            return;
        }
        let next_line_start = line_end + 1;
        let next_line_end = line_end_for(&self.text, next_line_start);
        let next_line_len = next_line_end - next_line_start;
        self.cursor_chars = next_line_start + current_column.min(next_line_len);
    }

    fn line_bounds(&self) -> (usize, usize) {
        let line_start = self.text[..self.cursor_byte_index()]
            .chars()
            .rev()
            .position(|ch| ch == '\n')
            .map(|offset| self.cursor_chars - offset)
            .unwrap_or(0);
        let line_end = line_end_for(&self.text, line_start);
        (line_start, line_end)
    }

    fn cursor_column(&self) -> usize {
        let (line_start, _) = self.line_bounds();
        self.cursor_chars.saturating_sub(line_start)
    }
}

impl From<&str> for ComposerState {
    fn from(value: &str) -> Self {
        Self {
            text: value.to_string(),
            cursor_chars: value.chars().count(),
        }
    }
}

fn char_to_byte_index(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .map(|(index, _)| index)
        .nth(char_index)
        .unwrap_or(text.len())
}

fn line_end_for(text: &str, line_start_chars: usize) -> usize {
    let suffix = &text[char_to_byte_index(text, line_start_chars)..];
    let line_len = suffix.chars().take_while(|ch| *ch != '\n').count();
    line_start_chars + line_len
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PendingRefresh {
    Sessions,
    Approvals,
    SessionView,
    Settings,
}

impl PendingRefresh {
    pub fn sessions() -> Self {
        Self::Sessions
    }

    pub fn session_view() -> Self {
        Self::SessionView
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackgroundResult {
    RefreshScheduled(PendingRefresh),
    RefreshCompleted(PendingRefresh),
    RefreshFailed {
        refresh: PendingRefresh,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopEvent {
    InsertChar(char),
    InsertNewline,
    Backspace,
    DeleteForward,
    MoveCursorLeft,
    MoveCursorRight,
    MoveCursorUp,
    MoveCursorDown,
    MoveCursorHome,
    MoveCursorEnd,
    SubmitComposer,
    SwitchScreen(Screen),
    CycleSessionNext,
    CycleSessionPrevious,
    RefreshNow,
    Resize,
    Tick,
    BackgroundResult(BackgroundResult),
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopCommand {
    SubmitChat { input: String },
    SwitchSession { session_id: String },
    RefreshAll,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopModel {
    active_screen: Screen,
    active_session_id: Option<String>,
    composer: ComposerState,
    session_order: Vec<String>,
    pending_refreshes: BTreeSet<PendingRefresh>,
    status_message: Option<String>,
}

impl Default for LoopModel {
    fn default() -> Self {
        Self {
            active_screen: Screen::Chat,
            active_session_id: None,
            composer: ComposerState::default(),
            session_order: Vec::new(),
            pending_refreshes: BTreeSet::new(),
            status_message: None,
        }
    }
}

impl LoopModel {
    pub fn composer(&self) -> &ComposerState {
        &self.composer
    }

    pub fn active_screen(&self) -> Screen {
        self.active_screen
    }

    pub fn active_session_id(&self) -> Option<&str> {
        self.active_session_id.as_deref()
    }

    pub fn pending_refreshes(&self) -> &BTreeSet<PendingRefresh> {
        &self.pending_refreshes
    }

    pub fn status_message(&self) -> Option<&str> {
        self.status_message.as_deref()
    }

    pub fn set_active_screen(&mut self, screen: Screen) {
        self.active_screen = screen;
    }

    pub fn set_active_session_id(&mut self, session_id: Option<String>) {
        self.active_session_id = session_id;
    }

    pub fn set_composer(&mut self, composer: ComposerState) {
        self.composer = composer;
    }

    pub fn set_session_order(&mut self, session_order: Vec<String>) {
        self.session_order = session_order;
    }

    pub fn set_status_message(&mut self, status_message: Option<String>) {
        self.status_message = status_message;
    }

    fn sync_from_app(&mut self, app: &App) {
        self.active_screen = app.state().active_screen();
        self.active_session_id = app.state().active_session_id().map(ToOwned::to_owned);
        self.session_order = app
            .state()
            .sessions()
            .iter()
            .map(|session| session.session_id.clone())
            .collect();
    }
}

pub fn reduce_event(
    model: &mut LoopModel,
    event: LoopEvent,
) -> Result<Option<LoopCommand>, String> {
    match event {
        LoopEvent::InsertChar(ch) => {
            model.composer.insert_char(ch);
            Ok(None)
        }
        LoopEvent::InsertNewline => {
            model.composer.insert_newline();
            Ok(None)
        }
        LoopEvent::Backspace => {
            model.composer.backspace();
            Ok(None)
        }
        LoopEvent::DeleteForward => {
            model.composer.delete_forward();
            Ok(None)
        }
        LoopEvent::MoveCursorLeft => {
            model.composer.move_left();
            Ok(None)
        }
        LoopEvent::MoveCursorRight => {
            model.composer.move_right();
            Ok(None)
        }
        LoopEvent::MoveCursorUp => {
            model.composer.move_up();
            Ok(None)
        }
        LoopEvent::MoveCursorDown => {
            model.composer.move_down();
            Ok(None)
        }
        LoopEvent::MoveCursorHome => {
            model.composer.move_home();
            Ok(None)
        }
        LoopEvent::MoveCursorEnd => {
            model.composer.move_end();
            Ok(None)
        }
        LoopEvent::SubmitComposer => {
            let input = model.composer.text.trim().to_string();
            if input.is_empty() {
                model.composer = ComposerState::default();
                return Ok(None);
            }
            model.composer = ComposerState::default();
            model
                .pending_refreshes
                .insert(PendingRefresh::session_view());
            Ok(Some(LoopCommand::SubmitChat { input }))
        }
        LoopEvent::SwitchScreen(screen) => {
            model.active_screen = screen;
            Ok(None)
        }
        LoopEvent::CycleSessionNext => Ok(cycle_session(model, true)),
        LoopEvent::CycleSessionPrevious => Ok(cycle_session(model, false)),
        LoopEvent::Resize => Ok(None),
        LoopEvent::RefreshNow | LoopEvent::Tick => Ok(Some(LoopCommand::RefreshAll)),
        LoopEvent::BackgroundResult(background) => {
            match background {
                BackgroundResult::RefreshScheduled(refresh) => {
                    model.pending_refreshes.insert(refresh);
                }
                BackgroundResult::RefreshCompleted(refresh) => {
                    model.pending_refreshes.remove(&refresh);
                }
                BackgroundResult::RefreshFailed { refresh, message } => {
                    model.pending_refreshes.remove(&refresh);
                    model.status_message = Some(sanitize_for_terminal(&message));
                }
            }
            Ok(None)
        }
        LoopEvent::Quit => Ok(None),
    }
}

fn cycle_session(model: &mut LoopModel, forward: bool) -> Option<LoopCommand> {
    if model.session_order.is_empty() {
        return None;
    }
    let current_index = model
        .active_session_id
        .as_ref()
        .and_then(|active| {
            model
                .session_order
                .iter()
                .position(|session| session == active)
        })
        .unwrap_or(0);
    let next_index = if forward {
        (current_index + 1) % model.session_order.len()
    } else if current_index == 0 {
        model.session_order.len() - 1
    } else {
        current_index - 1
    };
    model.active_session_id = Some(model.session_order[next_index].clone());
    Some(LoopCommand::SwitchSession {
        session_id: model.session_order[next_index].clone(),
    })
}

pub fn run(mut app: App) -> Result<(), String> {
    let mut terminal = TerminalSession::enter()?;
    let mut model = LoopModel::default();
    model.sync_from_app(&app);
    let mut last_tick = Instant::now();
    let (request_tx, result_rx) = spawn_worker();

    loop {
        drain_worker_results(&mut app, &mut model, &result_rx)?;
        apply_local_ui_state(&mut app, &model)?;
        model.sync_from_app(&app);
        render(&mut terminal.terminal, &app, &model)?;

        let timeout = next_loop_timeout(last_tick.elapsed(), !model.pending_refreshes().is_empty());
        if event::poll(timeout).map_err(|error| format!("terminal_poll_failed error={error}"))? {
            let event =
                event::read().map_err(|error| format!("terminal_read_failed error={error}"))?;
            if let Some(loop_event) = map_terminal_event(event) {
                if matches!(loop_event, LoopEvent::Quit) {
                    break;
                }
                if let Some(command) = reduce_event(&mut model, loop_event)? {
                    dispatch_command(&app, &mut model, &request_tx, command)?;
                } else {
                    apply_local_ui_state(&mut app, &model)?;
                }
            }
        }

        if last_tick.elapsed() >= TICK_INTERVAL
            && model.pending_refreshes().is_empty()
            && let Some(command) = reduce_event(&mut model, LoopEvent::Tick)?
        {
            dispatch_command(&app, &mut model, &request_tx, command)?;
        }
        if last_tick.elapsed() >= TICK_INTERVAL {
            last_tick = Instant::now();
        }
    }

    Ok(())
}

fn dispatch_command(
    app: &App,
    model: &mut LoopModel,
    request_tx: &Sender<WorkerRequest>,
    command: LoopCommand,
) -> Result<(), String> {
    for refresh in refreshes_for_command(&command) {
        reduce_event(
            model,
            LoopEvent::BackgroundResult(BackgroundResult::RefreshScheduled(refresh)),
        )?;
    }
    request_tx
        .send(WorkerRequest {
            app: app.clone(),
            command,
        })
        .map_err(|error| format!("tui_worker_send_failed error={error}"))
}

fn apply_local_ui_state(app: &mut App, model: &LoopModel) -> Result<(), String> {
    if app.state().active_screen() != model.active_screen() {
        app.state_mut().set_active_screen(model.active_screen());
    }
    app.apply_local_session_focus(model.active_session_id());
    Ok(())
}

fn drain_worker_results(
    app: &mut App,
    model: &mut LoopModel,
    result_rx: &Receiver<WorkerResult>,
) -> Result<(), String> {
    loop {
        match result_rx.try_recv() {
            Ok(result) => apply_worker_result(app, model, result)?,
            Err(TryRecvError::Empty) => return Ok(()),
            Err(TryRecvError::Disconnected) => {
                return Err("tui_worker_disconnected".to_string());
            }
        }
    }
}

fn apply_worker_result(
    app: &mut App,
    model: &mut LoopModel,
    result: WorkerResult,
) -> Result<(), String> {
    let WorkerResult {
        app: snapshot,
        status_message,
        completed_refreshes,
        failed_refreshes,
    } = result;

    if failed_refreshes.is_empty() {
        let active_screen = model.active_screen();
        app.apply_worker_snapshot(snapshot);
        app.state_mut().set_active_screen(active_screen);
        app.apply_local_session_focus(model.active_session_id());
    }

    model.sync_from_app(app);
    model.set_status_message(status_message);
    for refresh in completed_refreshes {
        reduce_event(
            model,
            LoopEvent::BackgroundResult(BackgroundResult::RefreshCompleted(refresh)),
        )?;
    }
    for (refresh, message) in failed_refreshes {
        reduce_event(
            model,
            LoopEvent::BackgroundResult(BackgroundResult::RefreshFailed { refresh, message }),
        )?;
    }
    Ok(())
}

fn next_loop_timeout(elapsed_since_tick: Duration, has_pending_work: bool) -> Duration {
    let tick_timeout = TICK_INTERVAL.saturating_sub(elapsed_since_tick);
    if has_pending_work {
        tick_timeout.min(PENDING_WORK_POLL_INTERVAL)
    } else {
        tick_timeout
    }
}

fn refreshes_for_command(command: &LoopCommand) -> Vec<PendingRefresh> {
    match command {
        LoopCommand::SubmitChat { .. } | LoopCommand::SwitchSession { .. } => {
            vec![PendingRefresh::session_view()]
        }
        LoopCommand::RefreshAll => vec![
            PendingRefresh::sessions(),
            PendingRefresh::Approvals,
            PendingRefresh::session_view(),
            PendingRefresh::Settings,
        ],
    }
}

#[derive(Clone)]
struct WorkerRequest {
    app: App,
    command: LoopCommand,
}

struct WorkerResult {
    app: App,
    status_message: Option<String>,
    completed_refreshes: Vec<PendingRefresh>,
    failed_refreshes: Vec<(PendingRefresh, String)>,
}

fn spawn_worker() -> (Sender<WorkerRequest>, Receiver<WorkerResult>) {
    let (request_tx, request_rx) = mpsc::channel::<WorkerRequest>();
    let (result_tx, result_rx) = mpsc::channel::<WorkerResult>();
    std::thread::spawn(move || {
        while let Ok(request) = request_rx.recv() {
            let result = execute_worker_request(request);
            if result_tx.send(result).is_err() {
                break;
            }
        }
    });
    (request_tx, result_rx)
}

fn execute_worker_request(request: WorkerRequest) -> WorkerResult {
    let mut app = request.app;
    match request.command {
        LoopCommand::SubmitChat { input } => match app.handle_chat_input(&input) {
            Ok(message) => WorkerResult {
                app,
                status_message: Some(message),
                completed_refreshes: vec![PendingRefresh::session_view()],
                failed_refreshes: Vec::new(),
            },
            Err(message) => WorkerResult {
                app,
                status_message: None,
                completed_refreshes: Vec::new(),
                failed_refreshes: vec![(PendingRefresh::session_view(), message)],
            },
        },
        LoopCommand::SwitchSession { session_id } => match app.switch_session(&session_id) {
            Ok(()) => WorkerResult {
                app,
                status_message: None,
                completed_refreshes: vec![PendingRefresh::session_view()],
                failed_refreshes: Vec::new(),
            },
            Err(message) => WorkerResult {
                app,
                status_message: None,
                completed_refreshes: Vec::new(),
                failed_refreshes: vec![(PendingRefresh::session_view(), message)],
            },
        },
        LoopCommand::RefreshAll => match app.refresh_dynamic_state() {
            Ok(()) => WorkerResult {
                app,
                status_message: None,
                completed_refreshes: vec![
                    PendingRefresh::sessions(),
                    PendingRefresh::Approvals,
                    PendingRefresh::session_view(),
                    PendingRefresh::Settings,
                ],
                failed_refreshes: Vec::new(),
            },
            Err(message) => WorkerResult {
                app,
                status_message: None,
                completed_refreshes: Vec::new(),
                failed_refreshes: vec![
                    (PendingRefresh::sessions(), message.clone()),
                    (PendingRefresh::Approvals, message.clone()),
                    (PendingRefresh::session_view(), message.clone()),
                    (PendingRefresh::Settings, message),
                ],
            },
        },
    }
}

fn map_terminal_event(event: CrosstermEvent) -> Option<LoopEvent> {
    match event {
        CrosstermEvent::Key(KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            ..
        }) => match (code, modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(LoopEvent::Quit),
            (KeyCode::Enter, KeyModifiers::CONTROL) => Some(LoopEvent::SubmitComposer),
            (KeyCode::Char('q'), KeyModifiers::NONE) => Some(LoopEvent::Quit),
            (KeyCode::Char('r'), KeyModifiers::CONTROL) => Some(LoopEvent::RefreshNow),
            (KeyCode::Char(ch), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                Some(LoopEvent::InsertChar(ch))
            }
            (KeyCode::Backspace, _) => Some(LoopEvent::Backspace),
            (KeyCode::Delete, _) => Some(LoopEvent::DeleteForward),
            (KeyCode::Left, _) => Some(LoopEvent::MoveCursorLeft),
            (KeyCode::Right, _) => Some(LoopEvent::MoveCursorRight),
            (KeyCode::Up, _) => Some(LoopEvent::MoveCursorUp),
            (KeyCode::Down, _) => Some(LoopEvent::MoveCursorDown),
            (KeyCode::Home, _) => Some(LoopEvent::MoveCursorHome),
            (KeyCode::End, _) => Some(LoopEvent::MoveCursorEnd),
            (KeyCode::Enter, _) => Some(LoopEvent::InsertNewline),
            (KeyCode::F(1), _) => Some(LoopEvent::SwitchScreen(Screen::Chat)),
            (KeyCode::F(2), _) => Some(LoopEvent::SwitchScreen(Screen::Hazel)),
            (KeyCode::F(3), _) => Some(LoopEvent::SwitchScreen(Screen::Sessions)),
            (KeyCode::F(4), _) => Some(LoopEvent::SwitchScreen(Screen::Approvals)),
            (KeyCode::F(5), _) => Some(LoopEvent::SwitchScreen(Screen::TraceArtifacts)),
            (KeyCode::F(6), _) => Some(LoopEvent::SwitchScreen(Screen::Settings)),
            (KeyCode::Tab, _) => Some(LoopEvent::CycleSessionNext),
            (KeyCode::BackTab, _) => Some(LoopEvent::CycleSessionPrevious),
            _ => None,
        },
        CrosstermEvent::Resize(_, _) => Some(LoopEvent::Resize),
        _ => None,
    }
}

fn render(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &App,
    model: &LoopModel,
) -> Result<(), String> {
    let status_message = model.status_message().map(sanitize_for_terminal);
    terminal
        .draw(|frame| render_frame(frame, app, model.composer(), status_message.as_deref()))
        .map_err(|error| format!("terminal_render_failed error={error}"))?;
    Ok(())
}

struct TerminalSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalSession {
    fn enter() -> Result<Self, String> {
        enable_raw_mode()
            .map_err(|error| format!("terminal_raw_mode_enable_failed error={error}"))?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen, Show)
            .map_err(|error| format!("terminal_enter_failed error={error}"))?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)
            .map_err(|error| format!("terminal_init_failed error={error}"))?;
        Ok(Self { terminal })
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = self.terminal.show_cursor();
        let _ = execute!(self.terminal.backend_mut(), Show, LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use sharo_core::protocol::{ApprovalSummary, SessionSummary, SessionView};

    use crate::app::{App, DaemonClient};

    use super::{
        BackgroundResult, LoopEvent, LoopModel, PENDING_WORK_POLL_INTERVAL, PendingRefresh,
        Screen, TICK_INTERVAL, WorkerResult, apply_local_ui_state, drain_worker_results,
        map_terminal_event, next_loop_timeout, reduce_event,
    };

    fn session_summary(session_id: &str) -> SessionSummary {
        SessionSummary {
            session_id: session_id.to_string(),
            session_label: format!("label-{session_id}"),
            session_status: "active".to_string(),
            activity_sequence: 1,
            latest_task_id: None,
            latest_task_state: None,
            latest_result_preview: None,
            has_pending_approval: false,
        }
    }

    fn session_view(session_id: &str) -> SessionView {
        SessionView {
            session_id: session_id.to_string(),
            session_label: format!("label-{session_id}"),
            tasks: Vec::new(),
            pending_approvals: Vec::new(),
            latest_result_preview: None,
            active_blocking_task_id: None,
        }
    }

    fn approval(approval_id: &str) -> ApprovalSummary {
        ApprovalSummary {
            approval_id: approval_id.to_string(),
            task_id: "task-1".to_string(),
            state: "pending".to_string(),
            reason: "needs approval".to_string(),
        }
    }

    #[test]
    fn screen_switch_persists_after_model_sync_roundtrip() {
        let client = DaemonClient::new("/tmp/sharo-daemon.sock");
        let mut app = App::new(client);
        let mut model = LoopModel::default();

        model.set_active_screen(Screen::Settings);
        apply_local_ui_state(&mut app, &model).expect("apply local ui state");
        model.sync_from_app(&app);

        assert_eq!(model.active_screen(), Screen::Settings);
        assert_eq!(app.state().active_screen(), Screen::Settings);
    }

    #[test]
    fn worker_snapshot_application_does_not_clobber_local_screen_focus() {
        let client = DaemonClient::new("/tmp/sharo-daemon.sock");
        let mut app = App::new(client.clone());
        let mut snapshot = App::new(client);
        app.state_mut().set_active_screen(Screen::Settings);
        snapshot.state_mut().set_active_screen(Screen::Chat);

        app.apply_worker_snapshot(snapshot);

        assert_eq!(app.state().active_screen(), Screen::Settings);
    }

    #[test]
    fn refresh_failure_messages_are_terminal_sanitized() {
        let mut model = LoopModel::default();

        reduce_event(
            &mut model,
            LoopEvent::BackgroundResult(BackgroundResult::RefreshFailed {
                refresh: PendingRefresh::Settings,
                message: "bad\x1b[31m\nmessage".to_string(),
            }),
        )
        .expect("apply refresh failure");

        let message = model.status_message().expect("status message");
        assert!(!message.chars().any(char::is_control));
        assert!(message.contains("\\u{1b}[31m"));
        assert!(message.contains("\\n"));
    }

    #[test]
    fn resize_terminal_event_is_normalized_into_loop_event() {
        let event = crossterm::event::Event::Resize(120, 40);

        let mapped = map_terminal_event(event);

        assert_eq!(mapped, Some(LoopEvent::Resize));
    }

    #[test]
    fn failed_worker_result_does_not_apply_partial_snapshot_state() {
        let client = DaemonClient::new("/tmp/sharo-daemon.sock");
        let mut app = App::new(client.clone());
        let mut model = LoopModel::default();
        app.state_mut()
            .set_active_session_id(Some("session-local".to_string()));
        app.state_mut().set_sessions(vec![session_summary("session-local")]);
        app.state_mut()
            .set_current_session_view(Some(session_view("session-local")));
        app.state_mut().set_approvals(vec![approval("approval-local")]);
        model.set_active_session_id(Some("session-local".to_string()));
        model.set_status_message(Some("before".to_string()));

        let mut snapshot = App::new(client);
        snapshot
            .state_mut()
            .set_sessions(vec![session_summary("session-remote")]);
        snapshot
            .state_mut()
            .set_current_session_view(Some(session_view("session-remote")));
        snapshot
            .state_mut()
            .set_approvals(vec![approval("approval-remote")]);

        let (tx, rx) = mpsc::channel();
        tx.send(WorkerResult {
            app: snapshot,
            status_message: None,
            completed_refreshes: vec![PendingRefresh::sessions()],
            failed_refreshes: vec![(
                PendingRefresh::Approvals,
                "approval refresh failed".to_string(),
            )],
        })
        .expect("send result");

        drain_worker_results(&mut app, &mut model, &rx).expect("drain results");

        assert_eq!(app.state().sessions(), &[session_summary("session-local")]);
        assert_eq!(
            app.state().current_session_view(),
            Some(&session_view("session-local"))
        );
        assert_eq!(app.state().approvals(), &[approval("approval-local")]);
        assert_eq!(model.active_session_id(), Some("session-local"));
        assert_eq!(
            model.status_message(),
            Some("approval refresh failed")
        );
    }

    #[test]
    fn stale_worker_snapshot_does_not_override_newer_local_session_focus() {
        let client = DaemonClient::new("/tmp/sharo-daemon.sock");
        let mut app = App::new(client.clone());
        let mut model = LoopModel::default();
        app.state_mut().set_active_session_id(Some("session-a".to_string()));
        app.state_mut()
            .set_current_session_view(Some(session_view("session-a")));
        model.set_active_session_id(Some("session-b".to_string()));

        let mut snapshot = App::new(client);
        snapshot
            .state_mut()
            .set_active_session_id(Some("session-a".to_string()));
        snapshot
            .state_mut()
            .set_current_session_view(Some(session_view("session-a")));

        let (tx, rx) = mpsc::channel();
        tx.send(WorkerResult {
            app: snapshot,
            status_message: None,
            completed_refreshes: vec![PendingRefresh::session_view()],
            failed_refreshes: Vec::new(),
        })
        .expect("send result");

        drain_worker_results(&mut app, &mut model, &rx).expect("drain results");

        assert_eq!(app.state().active_session_id(), Some("session-b"));
        assert_eq!(model.active_session_id(), Some("session-b"));
        assert_eq!(app.state().current_session_view(), None);
    }

    #[test]
    fn pending_background_work_uses_short_poll_timeout() {
        assert_eq!(
            next_loop_timeout(Duration::from_millis(10), true),
            PENDING_WORK_POLL_INTERVAL
        );
        assert_eq!(
            next_loop_timeout(TICK_INTERVAL, true),
            Duration::from_millis(0)
        );
    }
}
