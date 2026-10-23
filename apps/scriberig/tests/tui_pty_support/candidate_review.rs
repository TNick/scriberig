//! Drives the current shared candidate review and confirmation flow.

use std::time::Duration;

use super::TuiPty;

/// Opens the normal shared candidate review for the active agent result.
///
/// - `tui`: Active native TUI process.
/// - `validation`: Expected terminal validation status to display.
///
/// # Panics
/// Panics when the candidate review or expected validation status does not appear.
pub fn open_candidate_review(tui: &mut TuiPty, validation: &str) {
    tui.send_slow(b"/changes\r");
    let timeout = Duration::from_secs(120);
    assert!(
        tui.read_until_for("Candidate review", timeout),
        "{}",
        tui.diagnostic()
    );
    assert!(
        tui.read_until_for(validation, timeout),
        "candidate review did not finish loading: {}",
        tui.diagnostic()
    );
}

/// Opens the confirmation dialog for the active reviewed candidate.
///
/// - `tui`: Active native TUI process displaying candidate review.
///
/// # Panics
/// Panics when the confirmation dialog does not open after candidate review.
pub fn open_apply_confirmation(tui: &mut TuiPty) {
    tui.send(b"\x10");
    assert!(
        tui.read_until_for("Command palette", Duration::from_secs(120)),
        "command palette did not open: {}",
        tui.diagnostic()
    );
    tui.send_slow(b"Confirm");
    tui.send(b"\r");
    assert!(
        tui.read_until_for("Reviewed base:", Duration::from_secs(120)),
        "candidate confirmation dialog did not show its reviewed base: {}",
        tui.diagnostic()
    );
    assert!(
        tui.screen().contains("Confirm reviewed change"),
        "{}",
        tui.diagnostic()
    );
}

/// Accepts candidate confirmation and the resulting exact commit action.
///
/// - `tui`: Active native TUI process displaying candidate confirmation.
pub fn confirm_candidate_apply(tui: &mut TuiPty) {
    tui.send(b"\r");
}

/// Waits for the completed candidate operation to leave the confirmation dialog closed.
///
/// - `tui`: Active native TUI process after its candidate operation completed.
///
/// # Panics
/// Panics when the dialog remains visible after the PTY wait timeout.
pub fn assert_confirmation_closed(tui: &mut TuiPty) {
    assert!(
        tui.read_until_not("Confirm reviewed change"),
        "candidate confirmation did not close: {}",
        tui.diagnostic()
    );
}
