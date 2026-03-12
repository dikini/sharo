use sharo_tui::state::Screen;
use sharo_tui::tui_loop::{ComposerState, LoopCommand, LoopEvent, LoopModel, reduce_event};

#[test]
fn composer_editing_handles_multiline_insert_cursor_and_delete_boundaries() {
    let mut model = LoopModel::default();

    reduce_event(&mut model, LoopEvent::InsertChar('h')).expect("insert h");
    reduce_event(&mut model, LoopEvent::InsertChar('i')).expect("insert i");
    reduce_event(&mut model, LoopEvent::InsertNewline).expect("newline");
    reduce_event(&mut model, LoopEvent::InsertChar('x')).expect("insert x");
    reduce_event(&mut model, LoopEvent::MoveCursorLeft).expect("move left to newline");
    reduce_event(&mut model, LoopEvent::MoveCursorLeft).expect("move left to i");
    reduce_event(&mut model, LoopEvent::Backspace).expect("backspace removes i");
    reduce_event(&mut model, LoopEvent::DeleteForward).expect("delete removes newline");

    assert_eq!(model.composer().as_str(), "hx");
    assert_eq!(model.composer().cursor_chars(), 1);
}

#[test]
fn composer_submit_requires_ctrl_enter_and_plain_enter_inserts_newline() {
    let mut model = LoopModel::default();

    reduce_event(&mut model, LoopEvent::InsertChar('a')).expect("insert a");
    assert_eq!(
        reduce_event(&mut model, LoopEvent::InsertNewline).expect("plain enter"),
        None
    );
    reduce_event(&mut model, LoopEvent::InsertChar('b')).expect("insert b");

    assert_eq!(model.composer(), &ComposerState::from("a\nb"));
    assert_eq!(
        reduce_event(&mut model, LoopEvent::SubmitComposer).expect("ctrl-enter submit"),
        Some(LoopCommand::SubmitChat {
            input: "a\nb".to_string()
        })
    );
    assert_eq!(model.composer(), &ComposerState::default());
}

#[test]
fn screen_keybindings_switch_focus_without_losing_active_session() {
    let mut model = LoopModel::default();
    model.set_active_session_id(Some("session-42".to_string()));

    reduce_event(&mut model, LoopEvent::SwitchScreen(Screen::Settings)).expect("settings");
    assert_eq!(model.active_screen(), Screen::Settings);
    assert_eq!(model.active_session_id(), Some("session-42"));

    reduce_event(&mut model, LoopEvent::SwitchScreen(Screen::Approvals)).expect("approvals");
    assert_eq!(model.active_screen(), Screen::Approvals);
    assert_eq!(model.active_session_id(), Some("session-42"));
}
