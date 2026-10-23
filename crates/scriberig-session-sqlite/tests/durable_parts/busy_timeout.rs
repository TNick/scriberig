//! Verifies session writes tolerate short concurrent SQLite transactions.

use std::thread;
use std::time::Duration;

use rusqlite::Connection;
use scriberig_core::EntityId;
use scriberig_session::{Branch, Session, SessionExecutionSelection, SessionStore};
use scriberig_session_sqlite::SqliteSessionStore;

/// Waits for a short exclusive database lock instead of failing a session save.
#[test]
fn session_store_waits_for_a_short_sqlite_write_lock() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-session-busy-timeout-{}.sqlite",
        EntityId::new()
    ));
    let store = SqliteSessionStore::open(&path).expect("initialize session database");
    drop(store);
    let blocker = Connection::open(&path).expect("open lock connection");
    blocker
        .execute_batch("BEGIN EXCLUSIVE;")
        .expect("hold exclusive database lock");
    let writer_path = path.clone();
    let writer = thread::spawn(move || {
        let store = SqliteSessionStore::open(writer_path).expect("open concurrent session store");
        store
            .save(&Session {
                id: EntityId::new(),
                branches: vec![Branch::new(EntityId::new(), "root").expect("root")],
                goals: Vec::new(),
                tasks: Vec::new(),
                plans: Vec::new(),
                execution: SessionExecutionSelection::default(),
            })
            .expect("save after lock release");
    });

    thread::sleep(Duration::from_millis(100));
    blocker
        .execute_batch("COMMIT;")
        .expect("release database lock");
    writer.join().expect("session writer succeeds");
    drop(blocker);
    let _ = std::fs::remove_file(path);
}
