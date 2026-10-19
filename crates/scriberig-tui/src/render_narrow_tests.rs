//! Verifies the TUI keeps primary content visible at narrow widths.

use ratatui::{Terminal, backend::TestBackend};

use super::render;
use crate::state::TuiState;

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
    assert!(rendered.contains("No conversation yet"));
}
