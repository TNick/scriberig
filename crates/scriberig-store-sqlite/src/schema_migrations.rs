//! Applies additive SQLite columns for older persisted schemas.

use rusqlite::Transaction;

/// Adds optional columns introduced by supported schema revisions.
///
/// # Parameters
/// - `transaction`: Schema transaction receiving compatible column upgrades.
///
/// # Returns
/// Unit after every missing additive column has been created.
///
/// # Errors
/// Returns SQLite inspection or migration errors.
pub(super) fn upgrade_optional_columns(transaction: &Transaction<'_>) -> rusqlite::Result<()> {
    ensure_column(transaction, "project_entities", "name", "TEXT")?;
    ensure_column(
        transaction,
        "project_candidates",
        "validation_profile",
        "TEXT",
    )?;
    ensure_column(
        transaction,
        "project_candidates",
        "validation_digest",
        "BLOB",
    )?;
    ensure_column(
        transaction,
        "project_candidates",
        "semantic_operation",
        "TEXT",
    )?;
    ensure_column(transaction, "project_candidates", "semantic_target", "BLOB")?;
    ensure_column(
        transaction,
        "project_candidates",
        "semantic_changes",
        "TEXT",
    )?;
    ensure_column(transaction, "project_files", "grouping_key", "TEXT")?;
    ensure_column(
        transaction,
        "project_candidate_effects",
        "observed_bytes",
        "BLOB",
    )
}

/// Adds one column only when it is absent from an existing table.
///
/// # Parameters
/// - `transaction`: Schema transaction receiving the alteration.
/// - `table`: Static table name from this migration module.
/// - `column`: Static column name from this migration module.
/// - `kind`: Static SQLite column type.
///
/// # Returns
/// Unit when the column already exists or was added successfully.
///
/// # Errors
/// Returns an SQLite error when table inspection or alteration fails.
fn ensure_column(
    transaction: &Transaction<'_>,
    table: &str,
    column: &str,
    kind: &str,
) -> rusqlite::Result<()> {
    let columns = transaction
        .prepare(&format!("PRAGMA table_info({table})"))?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if !columns.iter().any(|existing| existing == column) {
        transaction.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {kind}"),
            [],
        )?;
    }
    Ok(())
}
