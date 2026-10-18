//! Verifies focused panel styles remain visible in the terminal renderer.

use ratatui::{Terminal, backend::TestBackend, layout::Rect, style::Color};

use super::render;
use crate::state::{Focus, TuiState};

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
