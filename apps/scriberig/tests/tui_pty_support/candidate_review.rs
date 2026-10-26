//! Drives the current shared candidate review and confirmation flow.

use std::time::{Duration, Instant};

use scriberig_core::{EntityId, SnapshotId};
use scriberig_tui::view::session::SessionModel;

use super::TuiPty;

/// Waits until the durable session projection can open the exact candidate review.
///
/// `/changes` reviews the newest projected candidate together with the base snapshot
/// recorded beside it, and the shared review operation rejects a request whose base is
/// not that candidate's exact base. A session read that lands before the durable
/// projection pairs the candidate with its base therefore cannot open this review, so
/// the fixture waits for the exact pair before it sends the command.
///
/// - `tui`: Active native TUI process owning the session database.
/// - `session`: Durable agent session carrying the candidate result.
/// - `candidate`: Candidate handle the review must select.
/// - `base_snapshot`: Exact base snapshot the review must carry.
///
/// # Panics
/// Panics when the durable projection never pairs that candidate with that base.
pub fn wait_for_durable_candidate_projection(
    tui: &TuiPty,
    session: EntityId,
    candidate: &str,
    base_snapshot: SnapshotId,
) {
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        let projection = SessionModel::from_session(&tui.load_session(session));
        let selected = projection
            .rows
            .iter()
            .rev()
            .find_map(|row| Some((row.candidate.clone()?, row.snapshot?)));
        if selected
            .as_ref()
            .is_some_and(|(handle, snapshot)| handle == candidate && *snapshot == base_snapshot)
        {
            return;
        }
        let projected = projection
            .rows
            .iter()
            .filter_map(|row| Some((row.candidate.clone()?, row.snapshot?)))
            .collect::<Vec<_>>();
        assert!(
            Instant::now() < deadline,
            "durable session projection never paired candidate {candidate} with exact base {base_snapshot}; projected candidates: {projected:?}"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

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
