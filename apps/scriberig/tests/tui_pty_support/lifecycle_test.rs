//! Verifies native TUI process lifecycle through a real pseudo-terminal.

use super::{TuiPty, test_lock};

/// Verifies the single-key quit command exits the native TUI cleanly.
#[test]
fn tui_pty_quits_with_q() {
    let _lock = test_lock();
    let mut tui = TuiPty::start();

    assert!(tui.read_until("project: "), "{}", tui.diagnostic());
    tui.send(b"q");
    assert!(tui.wait_for_exit());
}
