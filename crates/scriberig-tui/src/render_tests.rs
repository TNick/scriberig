//! Verifies the deterministic TUI renderer at representative sizes.

use ratatui::{Terminal, backend::TestBackend};

use super::render;
use crate::{
    explorer::{ExplorerModel, ExplorerRow},
    overlay::Overlay,
    selection::EntitySelection,
    state::{Focus, TuiState},
};

/// Renders representative sizes without touching a real terminal.
#[test]
fn renders_all_required_sizes() {
    for (width, height) in [(140, 40), (100, 30), (72, 24), (10, 3)] {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| render(frame, &TuiState::default()))
            .expect("draw");
    }
}

/// Shows the first-message conversation state without a fake session identity.
#[test]
fn default_frame_shows_conversation_home_and_persistent_composer() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal
        .draw(|frame| render(frame, &TuiState::default()))
        .expect("draw");
    let rendered = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect::<String>();

    assert!(rendered.contains("Conversation"));
    assert!(rendered.contains("New conversation"));
    assert!(rendered.contains("Type a message to start a session."));
    assert!(!rendered.contains("00000000-0000-0000-0000-000000000000"));
}

/// Shows a textual reply hint when the user pins an earlier activity.
#[test]
fn pinned_reply_target_has_a_textual_hint() {
    let mut state = TuiState::default();
    state.conversation_composer.target = crate::state::ConversationTarget::Existing {
        session: scriberig_core::EntityId::from_bytes([8; 16]),
        reply_anchor: scriberig_session::ContinuationAnchor::AssistantOutput {
            branch: scriberig_core::EntityId::from_bytes([9; 16]),
            turn: scriberig_core::EntityId::from_bytes([10; 16]),
        },
    };
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|frame| render(frame, &state)).expect("draw");
    let rendered = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect::<String>();

    assert!(rendered.contains("replying to earlier activity"));
}

/// Shows the current slash completion in the persistent composer.
#[test]
fn composer_renders_model_and_plan_command_suggestions() {
    for (draft, suggestion) in [("/mo", "/model"), ("/pl", "/plan")] {
        let mut state = TuiState::default();
        state.conversation_composer.draft = draft.to_owned();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal.draw(|frame| render(frame, &state)).expect("draw");
        let rendered = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();

        assert!(rendered.contains(suggestion), "{rendered}");
    }
}

/// Keeps OSC, BEL, ANSI, long, and malformed text inert in an auxiliary overlay.
#[test]
fn auxiliary_overlay_displays_control_sequences_as_safe_data() {
    let hostile = "\u{1b}[31mred\u{1b}]8;;https://invalid.test\u{7}link\u{1b}]8;;\u{7}\u{1b}]52;c;secret\u{7}";
    let mut state = TuiState {
        project: Some("project\u{1b}]0;title\u{7}".to_owned()),
        overlay: Some(Overlay::SessionList(vec![
            hostile.to_owned(),
            "a".repeat(2048),
            "**[malformed Markdown".to_owned(),
        ])),
        ..TuiState::default()
    };
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).expect("terminal");

    terminal
        .draw(|frame| render(frame, &state))
        .expect("draw hostile overlay");
    let rendered = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect::<String>();

    assert!(!rendered.chars().any(char::is_control));
    assert!(rendered.contains("\\u{1b}[31mred"));
    assert!(rendered.contains("\\u{7}link"));

    state.primary_view = crate::selection::PrimaryView::Explorer;
    state.overlay = None;
    state.explorer = ExplorerModel {
        root: Some(ExplorerRow {
            selection: EntitySelection {
                entity: scriberig_core::EntityId::from_bytes([121; 16]),
                revision: None,
                snapshot: None,
            },
            label: "red\u{1b}[31m".to_owned(),
            kind: Some("kind\u{1b}[2J".to_owned()),
            inspectable: true,
        }),
        ..ExplorerModel::default()
    };
    terminal
        .draw(|frame| render(frame, &state))
        .expect("draw hostile explorer");
    let rendered = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect::<String>();

    assert!(!rendered.chars().any(char::is_control));
    assert!(rendered.contains("red\\u{1b}[31m"), "{rendered}");
    assert!(rendered.contains("\\u{1b}[2J"));

    state.primary_view = crate::selection::PrimaryView::Session;
    state.explorer = ExplorerModel::default();
    state.session_composer = Some(crate::state::SessionComposerState {
        session: scriberig_core::EntityId::from_bytes([124; 16]),
        branch: None,
        text: hostile.to_owned(),
        pending_prompt: None,
        error: Some("error\u{1b}[2J".to_owned()),
    });
    terminal
        .draw(|frame| render(frame, &state))
        .expect("draw hostile session composer");
    let rendered = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect::<String>();

    assert!(!rendered.chars().any(char::is_control));
    assert!(rendered.contains("\\u{1b}[31mred"));
    assert!(rendered.contains("\\u{1b}[2J"));
}

/// Verifies rendered cells contain no raw terminal control characters.
///
/// - `backend`: Test backend whose visible cells are inspected.
fn assert_buffer_has_no_terminal_controls(backend: &TestBackend) {
    let rendered = backend
        .buffer()
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect::<String>();
    assert!(!rendered.chars().any(char::is_control), "{rendered}");
}

#[path = "render_tests/hostile.rs"]
mod hostile;
#[path = "render_tests/projection.rs"]
mod projection;
