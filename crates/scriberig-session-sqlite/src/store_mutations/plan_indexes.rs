//! Deletes session-owned derived plan indexes transactionally.

use rusqlite::{Transaction, params};
use scriberig_core::EntityId;

/// Deletes both rebuildable plan indexes for one removed session.
///
/// # Parameters
/// - `transaction`: Transaction deleting the session projection.
/// - `session`: Durable session whose derived rows are removed.
///
/// # Returns
/// Unit after both derived indexes have been cleared.
///
/// # Errors
/// Returns a SQLite diagnostic when either index cannot be cleared.
pub(super) fn delete_session_plan_indexes(
    transaction: &Transaction<'_>,
    session: EntityId,
) -> Result<(), String> {
    transaction
        .execute(
            "DELETE FROM plan_node_index WHERE session_id=?1",
            params![session.as_bytes().as_slice()],
        )
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "DELETE FROM plan_node_rollup_index WHERE session_id=?1",
            params![session.as_bytes().as_slice()],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}
