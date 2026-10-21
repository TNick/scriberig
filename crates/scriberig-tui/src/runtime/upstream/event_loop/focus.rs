//! Restores upstream focus after selector and help overlays close.

use std::sync::Arc;

use rpi_tui::{Component, Editor, TUI, TuiAltScreen};

/// Closes the active overlay and restores the prior focused component.
///
/// # Parameters
/// - `screen`: Upstream terminal runtime owning focus and the overlay stack.
/// - `editor`: Upstream editor whose cursor visibility follows focus.
/// - `_editor`: Retained editor argument for the shared focus restoration call.
/// - `focus_before_overlay`: Previous focused component to restore.
pub(in crate::runtime::upstream) fn restore_overlay_focus(
    screen: &TuiAltScreen,
    _editor: &Editor,
    focus_before_overlay: &mut Option<Arc<dyn Component>>,
) {
    screen.hide_overlay();
    screen.set_focus(focus_before_overlay.take());
}
