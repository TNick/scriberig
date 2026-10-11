//! Configures SQLite connections used by the durable storage backend.

use rusqlite::Connection;

/// Configures a connection for durable store operations.
///
/// # Parameters
/// - `connection`: SQLite connection to configure.
///
/// # Returns
/// The configured connection.
///
/// # Errors
/// Returns the SQLite configuration error.
pub fn configure(connection: Connection) -> rusqlite::Result<Connection> {
    connection.execute_batch("PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000;")?;
    Ok(connection)
}
