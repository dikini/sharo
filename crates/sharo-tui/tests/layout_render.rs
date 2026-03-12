use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use sharo_core::protocol::SessionView;
use sharo_tui::app::{App, DaemonClient};
use sharo_tui::layout::{composer_cursor_position, composer_height, render_frame};
use sharo_tui::state::Screen;
use sharo_tui::tui_loop::ComposerState;

fn buffer_to_string(buffer: &Buffer) -> String {
    let area = buffer.area();
    let mut out = String::new();
    for y in 0..area.height {
        for x in 0..area.width {
            out.push_str(buffer[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

#[test]
fn ratatui_layout_renders_active_screen_and_composer_regions() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut app = App::new(DaemonClient::new("/tmp/sharo-daemon.sock"));
    app.state_mut().set_active_screen(Screen::Settings);
    let composer = ComposerState::from("alpha\nbeta");

    terminal
        .draw(|frame| render_frame(frame, &app, &composer, Some("status ok")))
        .expect("draw");

    let rendered = buffer_to_string(terminal.backend().buffer());
    assert!(rendered.contains("Settings"));
    assert!(rendered.contains("alpha"));
    assert!(rendered.contains("beta"));
    assert!(rendered.contains("status ok"));
}

#[test]
fn layout_cursor_position_tracks_multiline_composer_state() {
    let composer = ComposerState::from("alpha\nbeta");
    let area = Rect::new(0, 20, 40, 4);

    let cursor = composer_cursor_position(area, &composer).expect("cursor");

    assert_eq!(cursor, (5, 22));
}

#[test]
fn composer_height_counts_trailing_blank_lines() {
    let composer = ComposerState::from("alpha\n");

    assert_eq!(composer_height(&composer), 4);
}

#[test]
fn ratatui_header_sanitizes_active_session_id() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut app = App::new(DaemonClient::new("/tmp/sharo-daemon.sock"));
    app.state_mut()
        .set_active_session_id(Some("session-\x1b[31mred".to_string()));
    app.state_mut().set_current_session_view(Some(SessionView {
        session_id: "session-\x1b[31mred".to_string(),
        session_label: "demo".to_string(),
        tasks: Vec::new(),
        pending_approvals: Vec::new(),
        latest_result_preview: None,
        active_blocking_task_id: None,
    }));

    terminal
        .draw(|frame| render_frame(frame, &app, &ComposerState::default(), None))
        .expect("draw");

    let rendered = buffer_to_string(terminal.backend().buffer());
    assert!(rendered.contains("session-\\u{1b}[31mred"));
    assert!(!rendered.contains("\x1b[31m"));
}
