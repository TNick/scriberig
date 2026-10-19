//! Configures SQLite connections used by the durable storage backend.

use rusqlite::Connection;

/// Configures a connection for durable store operations.
///
/// - `connection`: SQLite connection to configure.
///
/// The configured connection.
///
/// Returns the SQLite configuration error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn configure(connection: Connection) -> rusqlite::Result<Connection> {
    connection.execute_batch("PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000;")?;
    Ok(connection)
}
