//! Filters retained protocol events after an exact reconnect cursor.

use super::ApplicationError;
use scriberig_protocol::Event;

/// Filters events and detects cursors older than retained replay history.
///
/// - `events`: Ordered retained protocol events.
/// - `after`: Last sequence already consumed by the client.
/// - `super`: Parent server module containing event translation.
///
/// Events whose sequence is greater than `after`.
///
/// Returns `cursor_too_old` when the requested replay point has expired.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn filter_events_after(
    events: Vec<Event>,
    after: u64,
) -> Result<Vec<Event>, ApplicationError> {
    if after > 0
        && events
            .first()
            .is_some_and(|event| event.sequence > after.saturating_add(1))
    {
        let oldest = events.first().map_or(after, |event| event.sequence);
        return Err(ApplicationError::ProtocolFailure {
            category: "cursor_too_old".to_owned(),
            message: format!("event cursor {after} is older than retained cursor {oldest}"),
        });
    }
    Ok(events
        .into_iter()
        .filter(|event| event.sequence > after)
        .collect())
}
