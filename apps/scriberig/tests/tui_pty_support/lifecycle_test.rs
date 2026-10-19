//! Verifies native TUI process lifecycle through a real pseudo-terminal.

use super::{TuiPty, test_lock};

/// Verifies the explicit Ctrl-C quit command exits the native TUI cleanly.
#[test]
fn tui_pty_quits_with_ctrl_c() {
    let _lock = test_lock();
    let mut tui = TuiPty::start();

    assert!(tui.read_until("Conversation"), "{}", tui.diagnostic());
    tui.send(&[3]);
    assert!(tui.wait_for_exit());
}
