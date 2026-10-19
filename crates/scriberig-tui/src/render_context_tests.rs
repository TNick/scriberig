//! Verifies rendered project context, status, and composer surfaces.

use ratatui::{Terminal, backend::TestBackend, layout::Rect};

use super::render;
use crate::{
    overlay::Overlay,
    state::{Focus, TuiState, reduce},
};

/// Verifies the active project identity is visible in the context bar.
#[test]
fn context_bar_shows_active_project() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let state = TuiState {
        project: Some("prj_test".to_owned()),
        ..TuiState::default()
    };

    terminal.draw(|frame| render(frame, &state)).expect("draw");

    let buffer = terminal.backend().buffer();
    let layout = crate::layout::shell_layout(Rect::new(0, 0, 140, 40));
    let context: String = (layout.context.x..layout.context.right())
        .map(|x| buffer.cell((x, layout.context.y)).expect("cell").symbol())
        .collect();
    assert!(context.contains("prj_test"));
    assert!(!context.contains("focus:"));
    assert!(!context.contains("service:"));
}

/// Verifies the status line shows contextual shortcuts instead of shell state.
#[test]
fn status_line_shows_contextual_shortcuts() {
    let backend = TestBackend::new(72, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal
        .draw(|frame| render(frame, &TuiState::default()))
        .expect("draw");

    let buffer = terminal.backend().buffer();
    let layout = crate::layout::shell_layout(Rect::new(0, 0, 72, 24));
    let status: String = (layout.status.x..layout.status.right())
        .map(|x| buffer.cell((x, layout.status.y)).expect("cell").symbol())
        .collect();
    assert!(status.contains("Ctrl-C quit"));
    assert!(status.contains("Tab focus next"));
    assert!(!status.contains("operations: idle"));
}

/// Renders the persistent session composer beneath session activity.
#[test]
fn persistent_session_composer_is_visible_in_normal_surface() {
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let state = TuiState {
        primary_view: crate::selection::PrimaryView::Session,
        session_composer: Some(crate::state::SessionComposerState {
            session: scriberig_core::EntityId::from_bytes([41; 16]),
            branch: None,
            text: "review the current branch".to_owned(),
            pending_prompt: None,
            error: None,
        }),
        ..TuiState::default()
    };

    terminal.draw(|frame| render(frame, &state)).expect("draw");

    let rendered: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect();
    assert!(rendered.contains("Agent composer"));
    assert!(rendered.contains("review the current branch"));
}

/// Verifies opening an overlay clears shorter text from the previous frame.
#[test]
fn overlay_clears_stale_previous_frame_content() {
    let backend = TestBackend::new(72, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal
        .draw(|frame| render(frame, &TuiState::default()))
        .expect("initial draw");

    let mut state = TuiState::default();
    reduce(&mut state, crate::action::Action::PaletteOpened);
    terminal
        .draw(|frame| render(frame, &state))
        .expect("overlay draw");
    let rendered: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect();
    assert!(!rendered.contains("Search: ady"));
    assert!(matches!(state.overlay, Some(Overlay::Palette(_))));
}

/// Verifies palette navigation is visible on the selected command row.
#[test]
fn palette_renders_selected_row_marker() {
    let backend = TestBackend::new(72, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = TuiState::default();
    reduce(&mut state, crate::action::Action::PaletteOpened);
    reduce(
        &mut state,
        crate::action::Action::CursorMoved(crate::action::Direction::Next),
    );
    terminal
        .draw(|frame| render(frame, &state))
        .expect("overlay draw");
    let rendered: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect();
    assert!(rendered.contains("> Refresh"));
    assert!(!rendered.contains("> Open help"));
}

/// Verifies the selected explorer row remains visible below the viewport.
#[test]
fn explorer_scrolls_to_keep_selected_row_visible() {
    let backend = TestBackend::new(140, 12);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let root = scriberig_core::EntityId::from_bytes([1; 16]);
    let last = scriberig_core::EntityId::from_bytes([9; 16]);
    let mut state = TuiState {
        focus: Focus::Explorer,
        primary_view: crate::selection::PrimaryView::Explorer,
        explorer: crate::explorer::ExplorerModel::from_selection(
            crate::selection::EntitySelection {
                entity: root,
                revision: None,
                snapshot: None,
            },
        ),
        ..TuiState::default()
    };
    state.explorer.add_entities(
        (2_u8..=9).map(|value| scriberig_core::EntityId::from_bytes([value; 16])),
        None,
    );
    state.cursor = state.explorer.rows.len();

    terminal.draw(|frame| render(frame, &state)).expect("draw");

    let rendered: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect();
    let last_prefix = last.to_string();
    assert!(
        rendered.contains(&format!("> {}", &last_prefix[..12])),
        "{rendered}"
    );
}

/// Verifies explorer rows expose readable project context and semantic kind.
#[test]
fn explorer_renders_context_instead_of_only_stable_ids() {
    let backend = TestBackend::new(140, 12);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let entity = scriberig_core::EntityId::from_bytes([3; 16]);
    let mut state = TuiState {
        focus: Focus::Explorer,
        primary_view: crate::selection::PrimaryView::Explorer,
        explorer: crate::explorer::ExplorerModel::from_selection(
            crate::selection::EntitySelection {
                entity,
                revision: None,
                snapshot: None,
            },
        ),
        ..TuiState::default()
    };
    state.explorer.root.as_mut().expect("root").label = "src/app.py".to_owned();
    state.explorer.root.as_mut().expect("root").kind = Some("source".to_owned());

    terminal.draw(|frame| render(frame, &state)).expect("draw");

    let rendered: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect();
    assert!(rendered.contains("src/app.py [source]"), "{rendered}");
}
