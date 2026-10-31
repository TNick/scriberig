//! Implements the SQLite session repository contract.

use rusqlite::{TransactionBehavior, params};
use scriberig_core::{EntityId, SnapshotId};
use scriberig_session::{PlanStoreError, SelectionStoreError, Session, SessionStore};

use crate::{OptionalRow, SqliteSessionStore, branch_projection, payload, plan_indexes};

#[path = "repository/reads.rs"]
#[macro_use]
mod reads;
#[path = "repository/writes.rs"]
#[macro_use]
mod writes;

impl SessionStore for SqliteSessionStore {
    session_store_read_methods!();
    session_store_write_methods!();
}
