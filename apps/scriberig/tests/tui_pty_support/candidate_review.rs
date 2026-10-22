//! Drives the current shared candidate review and confirmation flow.

use super::TuiPty;

/// Opens the normal shared candidate review for the active agent result.
///
/// - `tui`: Active native TUI process.
pub fn open_candidate_review(tui: &mut TuiPty) {
    tui.send_slow(b"/changes\r");
    assert!(tui.read_until("Candidate review"), "{}", tui.diagnostic());
}

/// Opens the upstream confirmation dialog for a reviewed candidate.
///
/// - `tui`: Active native TUI process displaying candidate review.
pub fn open_apply_confirmation(tui: &mut TuiPty) {
    tui.send(b"\r");
    assert!(
        tui.read_until("Confirmation: required"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\x10");
    assert!(tui.read_until("Command palette"), "{}", tui.diagnostic());
    tui.send_slow(b"Confirm");
    assert!(
        tui.read_until("Confirm reviewed change"),
        "{}",
        tui.diagnostic()
    );
}

/// Accepts candidate confirmation and the resulting exact commit action.
///
/// - `tui`: Active native TUI process displaying candidate confirmation.
pub fn confirm_candidate_apply(tui: &mut TuiPty) {
    tui.send(b"\r");
    assert!(
        tui.read_until("Action: commit change"),
        "{}",
        tui.diagnostic()
    );
    tui.send(b"\r");
}
