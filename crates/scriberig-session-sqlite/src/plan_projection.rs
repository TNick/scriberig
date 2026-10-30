//! Persists ordered session-to-plan identity projections for semantic roots.

use rusqlite::{Connection, Transaction, TransactionBehavior, params};
use scriberig_core::EntityId;

use crate::SqliteSessionStore;

impl SqliteSessionStore {
    /// Registers a semantic plan root in the owning session's ordered projection.
    ///
    /// # Parameters
    /// - `session`: Owning session identity.
    /// - `plan`: Stable semantic plan identity.
    /// - `ordinal`: Plan order in the public session projection.
    ///
    /// # Returns
    /// Unit after the registry row is durable.
    ///
    /// # Errors
    /// Returns a SQLite diagnostic when the registry cannot be updated.
    pub(crate) fn register_plan(
        &self,
        session: EntityId,
        plan: EntityId,
        ordinal: usize,
    ) -> Result<(), String> {
        let ordinal = i64::try_from(ordinal).map_err(|_| "plan order exceeds SQLite range")?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "session repository lock poisoned".to_owned())?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        write(&transaction, session, plan, ordinal)?;
        transaction.commit().map_err(|error| error.to_string())
    }

    /// Loads stable plan identities in session presentation order.
    ///
    /// # Parameters
    /// - `session`: Owning session identity.
    ///
    /// # Returns
    /// Registered plan identities ordered by ordinal and stable ID.
    ///
    /// # Errors
    /// Returns a SQLite diagnostic when the registry cannot be read.
    pub(crate) fn registered_plans(&self, session: EntityId) -> Result<Vec<EntityId>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "session repository lock poisoned".to_owned())?;
        read(&connection, session)
    }
}

/// Upserts one plan identity inside an existing session transaction.
///
/// # Parameters
/// - `transaction`: Active SQLite transaction.
/// - `session`: Owning session identity.
/// - `plan`: Stable semantic plan identity.
/// - `ordinal`: Plan order in the public session projection.
///
/// # Returns
/// Unit after the plan identity is registered.
///
/// # Errors
/// Returns a SQLite diagnostic when the row cannot be written.
pub(crate) fn write(
    transaction: &Transaction<'_>,
    session: EntityId,
    plan: EntityId,
    ordinal: i64,
) -> Result<(), String> {
    transaction
        .execute(
            "INSERT INTO session_plan_records(session_id, plan_id, ordinal)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(session_id, plan_id) DO UPDATE SET ordinal=excluded.ordinal",
            params![
                session.as_bytes().as_slice(),
                plan.as_bytes().as_slice(),
                ordinal
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

/// Registers a plan using its position in the proposed session projection.
///
/// # Parameters
/// - `transaction`: Active SQLite transaction.
/// - `session`: Owning session identity.
/// - `plans`: Proposed ordered plan projections.
/// - `plan`: Stable semantic plan identity.
///
/// # Returns
/// Unit after the plan identity is registered.
///
/// # Errors
/// Returns an error when the plan order exceeds SQLite range or the row cannot
/// be written.
pub(crate) fn write_proposed_order(
    transaction: &Transaction<'_>,
    session: EntityId,
    plans: &[scriberig_session::Plan],
    plan: EntityId,
) -> Result<(), String> {
    let mut registered = false;
    for (ordinal, proposed) in plans.iter().enumerate() {
        let ordinal = i64::try_from(ordinal).map_err(|_| "plan order exceeds SQLite range")?;
        write(transaction, session, proposed.id, ordinal)?;
        registered |= proposed.id == plan;
    }
    if !registered {
        let ordinal = i64::try_from(plans.len()).map_err(|_| "plan order exceeds SQLite range")?;
        write(transaction, session, plan, ordinal)?;
    }
    Ok(())
}

/// Reads registered plan IDs from one SQLite connection.
///
/// # Parameters
/// - `connection`: SQLite connection containing the registry.
/// - `session`: Owning session identity.
///
/// # Returns
/// Plan IDs ordered by their persisted session order.
///
/// # Errors
/// Returns a SQLite diagnostic when a row cannot be decoded.
fn read(connection: &Connection, session: EntityId) -> Result<Vec<EntityId>, String> {
    let mut statement = connection
        .prepare(
            "SELECT plan_id FROM session_plan_records
             WHERE session_id=?1 ORDER BY ordinal, plan_id",
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
        .into_iter()
        .map(|bytes| {
            let bytes: [u8; 16] = bytes
                .try_into()
                .map_err(|_| "stored plan identity has invalid length".to_owned())?;
            Ok(EntityId::from_bytes(bytes))
        })
        .collect()
}
