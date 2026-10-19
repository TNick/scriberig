//! Recovery and transaction boundaries for SQLite durable writes.

use rusqlite::{Connection, Transaction};

/// Runs a closure in an explicit SQLite transaction.
///
/// - `connection`: Connection that owns the transaction.
/// - `operation`: Transactional operation.
///
/// The operation result after commit.
///
/// Returns the operation or commit error; dropping the transaction rolls back.
pub fn transaction<T, F>(connection: &mut Connection, operation: F) -> rusqlite::Result<T>
where
    F: for<'transaction> FnOnce(&'transaction Transaction<'transaction>) -> rusqlite::Result<T>,
{
    let transaction = connection.transaction()?;
    let result = operation(&transaction)?;
    transaction.commit()?;
    Ok(result)
}
