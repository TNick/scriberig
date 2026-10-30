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

/// Adds the exact-output validation digest when opening a version-five database.
#[test]
fn version_five_adds_candidate_validation_digest() {
    let mut connection = Connection::open_in_memory().expect("schema database");
    initialize(&mut connection).expect("initialize current schema");
    let transaction = connection.transaction().expect("begin downgrade fixture");
    transaction
        .execute(
            "ALTER TABLE project_candidates DROP COLUMN validation_digest",
            [],
        )
        .expect("remove version-six column");
    transaction
        .execute(
            "UPDATE metadata SET value='5' WHERE key='schema_version'",
            [],
        )
        .expect("mark version-five schema");
    transaction.commit().expect("commit version-five fixture");

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
    assert!(columns.iter().any(|column| column == "validation_digest"));
}

/// Adds persisted artifact grouping classification when opening a version-six database.
#[test]
fn version_six_adds_project_file_grouping_key() {
    let mut connection = Connection::open_in_memory().expect("schema database");
    initialize(&mut connection).expect("initialize current schema");
    let transaction = connection.transaction().expect("begin downgrade fixture");
    transaction
        .execute("ALTER TABLE project_files DROP COLUMN grouping_key", [])
        .expect("remove version-seven grouping key");
    transaction
        .execute(
            "UPDATE metadata SET value='6' WHERE key='schema_version'",
            [],
        )
        .expect("mark version-six schema");
    transaction.commit().expect("commit version-six fixture");

    assert_eq!(
        initialize(&mut connection).expect("upgrade schema"),
        CURRENT_SCHEMA_VERSION
    );
    let columns = connection
        .prepare("PRAGMA table_info(project_files)")
        .expect("project file table metadata")
        .query_map([], |row| row.get::<_, String>(1))
        .expect("project file columns")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("read project file columns");
    assert!(columns.iter().any(|column| column == "grouping_key"));
}

/// Adds semantic operation provenance when opening a version-seven database.
#[test]
fn version_seven_adds_candidate_operation_provenance() {
    let mut connection = Connection::open_in_memory().expect("schema database");
    initialize(&mut connection).expect("initialize current schema");
    let transaction = connection.transaction().expect("begin downgrade fixture");
    transaction
        .execute(
            "ALTER TABLE project_candidates DROP COLUMN semantic_operation",
            [],
        )
        .expect("remove version-eight operation field");
    transaction
        .execute(
            "UPDATE metadata SET value='7' WHERE key='schema_version'",
            [],
        )
        .expect("mark version-seven schema");
    transaction.commit().expect("commit version-seven fixture");

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
    assert!(columns.iter().any(|column| column == "semantic_operation"));
}

/// Adds a semantic target identity when opening a version-eight database.
#[test]
fn version_eight_adds_candidate_semantic_target() {
    let mut connection = Connection::open_in_memory().expect("schema database");
    initialize(&mut connection).expect("initialize current schema");
    let transaction = connection.transaction().expect("begin downgrade fixture");
    transaction
        .execute(
            "ALTER TABLE project_candidates DROP COLUMN semantic_target",
            [],
        )
        .expect("remove version-nine target field");
    transaction
        .execute(
            "UPDATE metadata SET value='8' WHERE key='schema_version'",
            [],
        )
        .expect("mark version-eight schema");
    transaction.commit().expect("commit version-eight fixture");

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
    assert!(columns.iter().any(|column| column == "semantic_target"));
}

/// Adds durable semantic candidate changes when opening a version-nine database.
#[test]
fn version_nine_adds_candidate_semantic_changes() {
    let mut connection = Connection::open_in_memory().expect("schema database");
    initialize(&mut connection).expect("initialize current schema");
    let transaction = connection.transaction().expect("begin downgrade fixture");
    transaction
        .execute(
            "ALTER TABLE project_candidates DROP COLUMN semantic_changes",
            [],
        )
        .expect("remove version-ten semantic changes field");
    transaction
        .execute(
            "UPDATE metadata SET value='9' WHERE key='schema_version'",
            [],
        )
        .expect("mark version-nine schema");
    transaction.commit().expect("commit version-nine fixture");

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
    assert!(columns.iter().any(|column| column == "semantic_changes"));
}

/// Adds observed live source bytes when opening a version-ten database.
#[test]
fn version_ten_adds_candidate_effect_observed_bytes() {
    let mut connection = Connection::open_in_memory().expect("schema database");
    initialize(&mut connection).expect("initialize current schema");
    let transaction = connection.transaction().expect("begin downgrade fixture");
    transaction
        .execute(
            "ALTER TABLE project_candidate_effects DROP COLUMN observed_bytes",
            [],
        )
        .expect("remove version-eleven observed bytes field");
    transaction
        .execute(
            "UPDATE metadata SET value='10' WHERE key='schema_version'",
            [],
        )
        .expect("mark version-ten schema");
    transaction.commit().expect("commit version-ten fixture");

    assert_eq!(
        initialize(&mut connection).expect("upgrade schema"),
        CURRENT_SCHEMA_VERSION
    );
    let columns = connection
        .prepare("PRAGMA table_info(project_candidate_effects)")
        .expect("candidate effect table metadata")
        .query_map([], |row| row.get::<_, String>(1))
        .expect("candidate effect columns")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("read candidate effect columns");
    assert!(columns.iter().any(|column| column == "observed_bytes"));
}
