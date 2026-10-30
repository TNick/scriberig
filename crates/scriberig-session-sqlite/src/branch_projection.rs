//! Persists session branch projections as independently replaceable rows.

use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use scriberig_core::EntityId;
use scriberig_session::{Branch, Session};

use crate::SqliteSessionStore;

/// Initializes or loads the normalized branch projection for one session.
///
/// Existing aggregate payloads seed this projection once. Later reads always
/// use the independently stored branch rows.
///
/// # Parameters
/// - `store`: Repository that owns the session connection.
/// - `session`: Decoded session whose branches seed a missing projection.
///
/// # Returns
/// Unit after replacing the decoded session's branch vector with stored rows.
///
/// # Errors
/// Returns a diagnostic when the projection cannot be initialized or decoded.
pub(crate) fn hydrate(store: &SqliteSessionStore, session: &mut Session) -> Result<(), String> {
    let mut connection = store
        .connection
        .lock()
        .map_err(|_| "session repository lock poisoned".to_owned())?;
    let marker = connection
        .query_row(
            "SELECT 1 FROM session_branch_meta WHERE session_id=?1",
            params![session.id.as_bytes().as_slice()],
            |_| Ok(()),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    if marker.is_none() {
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        initialize_if_missing(&transaction, session.id, &session.branches)?;
        transaction.commit().map_err(|error| error.to_string())?;
    }
    session.branches = read(&connection, session.id)?;
    Ok(())
}

/// Seeds legacy branches when their normalized projection has not been initialized.
///
/// # Parameters
/// - `transaction`: Atomic session mutation transaction.
/// - `session`: Owning session identity.
/// - `legacy`: Branches decoded from the prior aggregate payload.
///
/// Unit after the marker and initial rows exist.
///
/// # Returns
/// The operation result.
///
/// # Errors
/// Returns a diagnostic when branch serialization or SQLite writes fail.
pub(crate) fn initialize_if_missing(
    transaction: &Transaction<'_>,
    session: EntityId,
    legacy: &[Branch],
) -> Result<(), String> {
    if initialized(transaction, session)? {
        return Ok(());
    }
    for (ordinal, branch) in legacy.iter().enumerate() {
        upsert(transaction, session, branch, ordinal)?;
    }
    transaction
        .execute(
            "INSERT INTO session_branch_meta(session_id) VALUES (?1)",
            params![session.as_bytes().as_slice()],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

/// Replaces every normalized branch row for a whole-session save.
///
/// # Parameters
/// - `transaction`: Atomic session mutation transaction.
/// - `session`: Owning session identity.
/// - `branches`: Complete branch projection to persist.
///
/// Unit after the normalized projection matches the supplied session.
///
/// # Returns
/// The operation result.
///
/// # Errors
/// Returns a diagnostic when serialization or SQLite writes fail.
pub(crate) fn replace_all(
    transaction: &Transaction<'_>,
    session: EntityId,
    branches: &[Branch],
) -> Result<(), String> {
    transaction
        .execute(
            "DELETE FROM session_branch_records WHERE session_id=?1",
            params![session.as_bytes().as_slice()],
        )
        .map_err(|error| error.to_string())?;
    for (ordinal, branch) in branches.iter().enumerate() {
        upsert(transaction, session, branch, ordinal)?;
    }
    transaction
        .execute(
            "INSERT INTO session_branch_meta(session_id) VALUES (?1)
             ON CONFLICT(session_id) DO NOTHING",
            params![session.as_bytes().as_slice()],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

/// Loads branch rows in stable session order.
///
/// # Parameters
/// - `connection`: SQLite connection containing normalized branch rows.
/// - `session`: Owning session identity.
///
/// # Returns
/// All branches ordered by their persisted session ordinal.
///
/// # Errors
/// Returns a diagnostic when a stored branch cannot be decoded.
pub(crate) fn read(connection: &Connection, session: EntityId) -> Result<Vec<Branch>, String> {
    let mut statement = connection
        .prepare(
            "SELECT payload FROM session_branch_records
             WHERE session_id=?1 ORDER BY ordinal, branch_id",
        )
        .map_err(|error| error.to_string())?;
    let records = statement
        .query_map(params![session.as_bytes().as_slice()], |row| {
            row.get::<_, Vec<u8>>(0)
        })
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;
    records
        .iter()
        .map(|record| serde_json::from_slice(record).map_err(|error| error.to_string()))
        .collect()
}

/// Writes one changed branch without touching unrelated branch rows.
///
/// # Parameters
/// - `connection`: SQLite connection or transaction with branch tables.
/// - `session`: Owning session identity.
/// - `branch`: Branch revision to persist.
/// - `ordinal`: Stable position in the session branch projection.
///
/// Unit after the branch row is inserted or replaced.
///
/// # Returns
/// The operation result.
///
/// # Errors
/// Returns a diagnostic when serialization or SQLite writes fail.
pub(crate) fn upsert(
    connection: &Connection,
    session: EntityId,
    branch: &Branch,
    ordinal: usize,
) -> Result<(), String> {
    let payload = serde_json::to_vec(branch).map_err(|error| error.to_string())?;
    let ordinal = i64::try_from(ordinal).map_err(|_| "branch order exceeds SQLite range")?;
    connection
        .execute(
            "INSERT INTO session_branch_records(session_id, branch_id, ordinal, payload)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(session_id, branch_id) DO UPDATE SET
               ordinal=excluded.ordinal, payload=excluded.payload",
            params![
                session.as_bytes().as_slice(),
                branch.id.as_bytes().as_slice(),
                ordinal,
                payload
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

/// Checks whether branch rows have an initialized authority marker.
///
/// # Parameters
/// - `connection`: SQLite connection containing branch metadata.
/// - `session`: Owning session identity.
///
/// # Returns
/// `true` when normalized branch rows are authoritative for the session.
///
/// # Errors
/// Returns a diagnostic when SQLite cannot query the metadata row.
fn initialized(connection: &Connection, session: EntityId) -> Result<bool, String> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM session_branch_meta WHERE session_id=?1)",
            params![session.as_bytes().as_slice()],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())
}
