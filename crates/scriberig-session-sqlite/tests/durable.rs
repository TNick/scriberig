//! Verifies session graphs survive repository reopen and preserve child state.

use scriberig_core::EntityId;
use scriberig_session::{Branch, Session, SessionStore};
use scriberig_session_sqlite::SqliteSessionStore;

/// Verifies a complete session graph round-trips through SQLite.
#[test]
fn session_graph_survives_reopen() {
    let path = std::env::temp_dir().join(format!("scriberig-session-{}.sqlite", EntityId::new()));
    let session_id = EntityId::from_bytes([1; 16]);
    let root_id = EntityId::from_bytes([2; 16]);
    let child_id = EntityId::from_bytes([3; 16]);
    {
        let store = SqliteSessionStore::open(&path).expect("open store");
        let mut session = Session {
            id: session_id,
            branches: vec![Branch::new(root_id, "root").expect("root")],
            goals: Vec::new(),
            tasks: Vec::new(),
        };
        session
            .add_child_branch(child_id, root_id, "child", "inspect")
            .expect("child");
        store.save(&session).expect("save session");
    }
    let reopened = SqliteSessionStore::open(&path).expect("reopen store");
    let loaded = reopened
        .load(session_id)
        .expect("load session")
        .expect("stored session");
    assert_eq!(loaded.id, session_id);
    assert_eq!(loaded.branches.len(), 2);
    assert_eq!(loaded.branches[0].children, vec![child_id]);
    let _ = std::fs::remove_file(path);
}
