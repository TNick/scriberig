//! Verifies the deterministic TUI renderer at representative sizes.

use ratatui::{Terminal, backend::TestBackend, layout::Rect, style::Color};

use super::render;
use crate::{
    overlay::Overlay,
    state::{Focus, TuiState, reduce},
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

/// Verifies the narrow shell preserves a readable primary message.
#[test]
fn narrow_frame_contains_primary_text() {
    let backend = TestBackend::new(72, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal
        .draw(|frame| render(frame, &TuiState::default()))
        .expect("draw");
    let buffer = terminal.backend().buffer();
    let rendered: String = buffer
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect();
    assert!(rendered.contains("Shell ready"));
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

/// Verifies the focused panel uses the accent border color.
#[test]
fn focused_panel_has_accent_border() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let state = TuiState {
        focus: Focus::Explorer,
        ..TuiState::default()
    };
    terminal.draw(|frame| render(frame, &state)).expect("draw");

    let buffer = terminal.backend().buffer();
    let layout = crate::layout::shell_layout(Rect::new(0, 0, 140, 40));
    let explorer = layout.explorer.expect("explorer");
    let primary = layout.primary;
    assert_eq!(
        buffer.cell((explorer.x, explorer.y)).expect("cell").fg,
        Color::Cyan
    );
    assert_eq!(
        buffer.cell((primary.x, primary.y)).expect("cell").fg,
        Color::DarkGray
    );
}
