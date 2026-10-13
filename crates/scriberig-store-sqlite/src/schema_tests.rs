//! Verifies additive SQLite backend schema upgrades.

use super::{CURRENT_SCHEMA_VERSION, initialize};
use rusqlite::Connection;

/// Adds the candidate profile field when opening a version-four database.
#[test]
fn version_four_adds_candidate_validation_profile() {
    let mut connection = Connection::open_in_memory().expect("schema database");
    initialize(&mut connection).expect("initialize current schema");
    let transaction = connection.transaction().expect("begin downgrade fixture");
    transaction
        .execute(
            "ALTER TABLE project_candidates DROP COLUMN validation_profile",
            [],
        )
        .expect("remove version-five column");
    transaction
        .execute(
            "UPDATE metadata SET value='4' WHERE key='schema_version'",
            [],
        )
        .expect("mark version-four schema");
    transaction.commit().expect("commit version-four fixture");

    assert_eq!(
        initialize(&mut connection).expect("upgrade schema"),
        CURRENT_SCHEMA_VERSION
    );
    let columns = connection
        .prepare("PRAGMA table_info(project_candidates)")
        .expect("candidate table metadata")
        .query_map([], |row| row.get::<_, String>(1))
        .expect("candidate columns")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("read candidate columns");
    assert!(columns.iter().any(|column| column == "validation_profile"));
}
