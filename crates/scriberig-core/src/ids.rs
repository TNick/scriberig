//! Re-exports strongly typed identifiers and validated names.

// Keep identifier implementations focused in small modules.
mod common;
mod digest;
mod entity;
mod names;

// Expose identifier types through the stable ids module.
pub use common::IdParseError;
pub use digest::{RevisionId, SnapshotId};
pub use entity::EntityId;
pub use names::{ObjectType, RootName};
