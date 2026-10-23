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

/// Opens the upstream confirmation dialog for a reviewed candidate.
///
/// - `tui`: Active native TUI process displaying candidate review.
///
/// # Panics
/// Panics when the confirmation dialog does not open after candidate review.
pub fn open_apply_confirmation(tui: &mut TuiPty) {
    tui.send(b"\x1b[18~");
    assert!(
        tui.read_until_next("Validation: Passed"),
        "candidate review did not regain focus: {}",
        tui.diagnostic()
    );
    tui.send(b"\r");
    assert!(
        tui.read_until("Confirm reviewed change"),
        "{}",
        tui.diagnostic()
    );
}

/// Accepts candidate confirmation and the resulting exact commit action.
///
/// - `tui`: Active native TUI process displaying candidate confirmation.
///
/// # Panics
/// Panics when the confirmation dialog remains open after acceptance.
pub fn confirm_candidate_apply(tui: &mut TuiPty) {
    tui.send(b"\r");
    assert!(
        tui.read_until_not("Confirm reviewed change"),
        "candidate confirmation did not close: {}",
        tui.diagnostic()
    );
}
