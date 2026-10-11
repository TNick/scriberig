//! Defines backend-independent immutable revision, snapshot, and root storage contracts.

pub mod revision;
pub mod revision_store;

pub use revision::Revision;
pub use revision_store::{RevisionStore, StoreError};
