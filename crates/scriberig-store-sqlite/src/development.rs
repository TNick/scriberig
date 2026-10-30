//! Provides development database initialization for the SQLite backend.

use std::path::Path;

use crate::SqliteStore;

/// Initializes a development database path and its schema.
///
/// # Parameters
/// - `path`: SQLite database path.
///
/// # Returns
/// Unit after initialization.
///
/// # Errors
/// Returns an initialization error when SQLite rejects the database.
pub fn initialize_development_database(
    path: impl AsRef<Path>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    SqliteStore::open(path).map(|_| ()).map_err(Into::into)
}
