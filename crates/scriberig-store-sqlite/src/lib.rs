//! Provides the durable SQLite implementation of the Scriberig storage contracts.

use std::error::Error;
use std::fs;
use std::path::Path;

/// Create the development database and its initial SQLite metadata.
///
/// # Parameters
/// - `path`: File path where SQLite should create or open the database.
///
/// # Returns
/// Unit after the database metadata is initialized.
///
/// # Errors
/// Returns an error when the directory or SQLite database cannot be initialized.
pub fn initialize_development_database(
    path: impl AsRef<Path>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let connection = rusqlite::Connection::open(path)?;
    connection.execute_batch("PRAGMA journal_mode = WAL; PRAGMA user_version = 1;")?;
    Ok(())
}
