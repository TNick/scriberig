//! Proves style settings survive a restart and keep optimistic concurrency.

use scriberig_app::{ApplicationError, ProjectionStyleStore, StyleEntryKey, StyleScopeKind};
use scriberig_style_sqlite::SqliteProjectionStyleStore;
use sr_projection::{ProjectionKind, StyleValue};

/// Builds one projection kind label for these tests.
///
/// - `label`: Kind label to use.
///
/// # Returns
/// The validated projection kind.
fn kind(label: &str) -> ProjectionKind {
    ProjectionKind::new(label).expect("kind label is valid")
}

/// Builds one user-scope configuration key.
///
/// - `principal`: Principal whose preference is stored.
///
/// # Returns
/// The configuration key.
fn user_key(principal: &str) -> StyleEntryKey {
    StyleEntryKey::new(StyleScopeKind::User, principal, kind("rust-source"))
}

/// Verifies a setting written by one store is readable after reopening the file.
#[test]
fn a_setting_written_by_one_process_survives_reopening_the_database() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("scriberig.db");
    let key = user_key("alice");

    {
        let store = SqliteProjectionStyleStore::open(&path).expect("style store opens");
        let stored = store
            .compare_and_swap(&key, None, StyleValue::new(b"{\"width\":100}".to_vec()))
            .expect("first write");
        assert_eq!(stored.revision, 1);
    }

    let reopened = SqliteProjectionStyleStore::open(&path).expect("style store reopens");
    let stored = reopened
        .read(&key)
        .expect("durable read")
        .expect("setting survived");
    assert_eq!(stored.value.as_bytes(), b"{\"width\":100}");
    assert_eq!(stored.revision, 1);
    assert_eq!(
        reopened
            .read(&StyleEntryKey::new(
                StyleScopeKind::User,
                "bob",
                kind("rust-source")
            ))
            .expect("other principal read"),
        None,
        "one principal's setting must not leak into another's scope"
    );
}

/// Verifies a workspace materialization policy is durable beside a preference.
#[test]
fn each_scope_keeps_its_own_durable_setting() {
    let store = SqliteProjectionStyleStore::open_in_memory().expect("in-memory store");
    let user = user_key("alice");
    let workspace = StyleEntryKey::new(StyleScopeKind::Workspace, "ws-1", kind("rust-source"));

    store
        .compare_and_swap(&user, None, StyleValue::new(b"user".to_vec()))
        .expect("user setting");
    store
        .compare_and_swap(&workspace, None, StyleValue::new(b"workspace".to_vec()))
        .expect("workspace setting");

    assert_eq!(
        store.read(&user).expect("read").unwrap().value.as_bytes(),
        b"user"
    );
    assert_eq!(
        store
            .read(&workspace)
            .expect("read")
            .unwrap()
            .value
            .as_bytes(),
        b"workspace"
    );
}

/// Verifies revisions increase monotonically and stale writes are refused.
#[test]
fn a_stale_revision_is_refused_and_a_current_revision_advances_it() {
    let store = SqliteProjectionStyleStore::open_in_memory().expect("in-memory store");
    let key = user_key("alice");

    store
        .compare_and_swap(&key, None, StyleValue::new(b"first".to_vec()))
        .expect("first write");
    let error = store
        .compare_and_swap(&key, Some(0), StyleValue::new(b"stale".to_vec()))
        .expect_err("a stale revision must not write");
    assert!(
        matches!(error, ApplicationError::SemanticConflict { .. }),
        "{error:?}"
    );

    let written = store
        .compare_and_swap(&key, Some(1), StyleValue::new(b"second".to_vec()))
        .expect("current revision writes");
    assert_eq!(written.revision, 2);
    assert_eq!(
        store.read(&key).expect("read").unwrap().value.as_bytes(),
        b"second"
    );

    store
        .clear(&key, Some(2))
        .expect("clear at current revision");
    assert_eq!(store.read(&key).expect("read"), None);
    let error = store
        .clear(&key, Some(2))
        .expect_err("clearing twice is stale");
    assert!(
        matches!(error, ApplicationError::SemanticConflict { .. }),
        "{error:?}"
    );
}
