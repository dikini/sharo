use sharo_tui::state::Screen;
use sharo_tui::tui_loop::{
    BackgroundResult, ComposerState, LoopCommand, LoopEvent, LoopModel, PendingRefresh,
    reduce_event,
};

#[test]
fn background_result_events_are_applied_only_through_main_loop_reducer() {
    let mut model = LoopModel::default();
    let refresh = PendingRefresh::sessions();

    reduce_event(
        &mut model,
        LoopEvent::BackgroundResult(BackgroundResult::RefreshScheduled(refresh.clone())),
    )
    .expect("schedule refresh");
    assert!(model.pending_refreshes().contains(&refresh));

    reduce_event(
        &mut model,
        LoopEvent::BackgroundResult(BackgroundResult::RefreshCompleted(refresh.clone())),
    )
    .expect("complete refresh");
    assert!(!model.pending_refreshes().contains(&refresh));
}

#[test]
fn interactive_loop_submits_chat_and_refreshes_view() {
    let mut model = LoopModel::default();
    model.set_active_screen(Screen::Chat);
    model.set_composer(ComposerState::from("read one context item"));

    let command = reduce_event(&mut model, LoopEvent::SubmitComposer)
        .expect("submit")
        .expect("command");

    assert_eq!(
        command,
        LoopCommand::SubmitChat {
            input: "read one context item".to_string()
        }
    );
    assert_eq!(model.composer(), &ComposerState::default());
    assert!(
        model
            .pending_refreshes()
            .contains(&PendingRefresh::session_view())
    );
}

#[test]
fn interactive_loop_multiline_cursor_navigation_preserves_buffer_integrity() {
    let mut model = LoopModel::default();
    model.set_composer(ComposerState::from("alpha\nbeta"));

    reduce_event(&mut model, LoopEvent::MoveCursorHome).expect("home");
    reduce_event(&mut model, LoopEvent::MoveCursorDown).expect("down");
    reduce_event(&mut model, LoopEvent::MoveCursorEnd).expect("end");
    reduce_event(&mut model, LoopEvent::MoveCursorUp).expect("up");
    reduce_event(&mut model, LoopEvent::InsertChar('!')).expect("insert");

    assert_eq!(model.composer().as_str(), "alph!a\nbeta");
    assert_eq!(model.composer().cursor_chars(), 5);
}

#[test]
fn interactive_loop_cycles_sessions_without_cross_contamination() {
    let mut model = LoopModel::default();
    model.set_session_order(vec!["session-a".to_string(), "session-b".to_string()]);
    model.set_active_session_id(Some("session-a".to_string()));

    reduce_event(&mut model, LoopEvent::CycleSessionNext).expect("cycle next");
    assert_eq!(model.active_session_id(), Some("session-b"));

    reduce_event(&mut model, LoopEvent::CycleSessionPrevious).expect("cycle previous");
    assert_eq!(model.active_session_id(), Some("session-a"));
}

#[test]
fn interactive_loop_refresh_tick_preserves_consistent_state_on_fetch_failure() {
    let mut model = LoopModel::default();
    model.set_active_session_id(Some("session-42".to_string()));
    model.set_active_screen(Screen::Chat);
    let before = model.clone();

    reduce_event(
        &mut model,
        LoopEvent::BackgroundResult(BackgroundResult::RefreshFailed {
            refresh: PendingRefresh::session_view(),
            message: "daemon_unavailable".to_string(),
        }),
    )
    .expect("refresh failure");

    assert_eq!(model.active_session_id(), before.active_session_id());
    assert_eq!(model.active_screen(), before.active_screen());
    assert_eq!(model.composer(), before.composer());
}
