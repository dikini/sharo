use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::App;
use crate::screens::sanitize_for_terminal;
use crate::state::Screen;
use crate::tui_loop::ComposerState;

pub fn render_frame(
    frame: &mut Frame<'_>,
    app: &App,
    composer: &ComposerState,
    status: Option<&str>,
) {
    let areas = layout_areas(frame.area(), composer);

    let header = Paragraph::new(header_text(app))
        .block(Block::default().borders(Borders::ALL).title("Sharo TUI"));
    frame.render_widget(header, areas.header);

    let body = Paragraph::new(active_screen_text(app))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(app.state().active_screen().title()),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(body, areas.body);

    let status_text = status.unwrap_or("ready");
    let status_widget =
        Paragraph::new(status_text).block(Block::default().borders(Borders::ALL).title("Status"));
    frame.render_widget(status_widget, areas.status);

    let composer_widget = Paragraph::new(composer.as_str())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Line::from(vec![
                    "Composer ".into(),
                    "(Ctrl-Enter submits)".into(),
                ])),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(composer_widget, areas.composer);
    if let Some((x, y)) = composer_cursor_position(areas.composer, composer) {
        frame.set_cursor_position((x, y));
    }
}

pub fn composer_cursor_position(area: Rect, composer: &ComposerState) -> Option<(u16, u16)> {
    if area.width <= 2 || area.height <= 2 {
        return None;
    }
    let text = composer.as_str();
    let mut x = area.x.saturating_add(1);
    let mut y = area.y.saturating_add(1);
    let min_x = area.x.saturating_add(1);
    let max_x = area.x.saturating_add(area.width.saturating_sub(2));
    let max_y = area.y.saturating_add(area.height.saturating_sub(2));

    for ch in text.chars().take(composer.cursor_chars()) {
        if ch == '\n' {
            y = y.saturating_add(1).min(max_y);
            x = min_x;
            continue;
        }
        if x < max_x {
            x = x.saturating_add(1);
        } else {
            y = y.saturating_add(1).min(max_y);
            x = min_x;
        }
    }
    Some((x, y))
}

pub fn composer_height(composer: &ComposerState) -> u16 {
    let lines = composer.as_str().split('\n').count().max(1) as u16;
    (lines + 2).clamp(3, 8)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FrameAreas {
    header: Rect,
    body: Rect,
    status: Rect,
    composer: Rect,
}

fn layout_areas(frame_area: Rect, composer: &ComposerState) -> FrameAreas {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(8),
        Constraint::Length(3),
        Constraint::Length(composer_height(composer)),
    ])
    .split(frame_area);
    FrameAreas {
        header: chunks[0],
        body: chunks[1],
        status: chunks[2],
        composer: chunks[3],
    }
}

fn header_text(app: &App) -> Text<'_> {
    Text::from(vec![
        Line::from(vec![
            "screen: ".into(),
            app.state().active_screen().title().into(),
            " | session: ".into(),
            sanitize_for_terminal(app.state().active_session_id().unwrap_or("none")).into(),
        ]),
        Line::from(vec![
            "keys: ".into(),
            "F1-F6 screens | Tab next session | Shift-Tab prev | Ctrl-R refresh | q quit".into(),
        ]),
    ])
}

fn active_screen_text(app: &App) -> Text<'_> {
    let raw = match app.state().active_screen() {
        Screen::Chat => app.render_chat(),
        Screen::Hazel => app.render_hazel(),
        Screen::Sessions => app.render_sessions(),
        Screen::Approvals => app.render_approvals(),
        Screen::TraceArtifacts => app.render_trace_artifacts(),
        Screen::Settings => app.render_settings(),
    };

    let title = match app.state().active_screen() {
        Screen::Chat => "Chat",
        Screen::Hazel => "Hazel",
        Screen::Sessions => "Sessions",
        Screen::Approvals => "Approvals",
        Screen::TraceArtifacts => "Trace/Artifacts",
        Screen::Settings => "Settings",
    };

    Text::from(vec![
        Line::styled(title, Style::default().add_modifier(Modifier::BOLD)),
        Line::raw(""),
        Line::raw(raw),
    ])
}
