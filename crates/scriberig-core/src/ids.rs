//! Re-exports strongly typed identifiers and validated names.

// Keep identifier implementations focused in small modules.
mod common;
mod digest;
mod entity;
mod names;
mod operation;
mod project;
mod type_id;

// Expose identifier types through the stable ids module.
pub use common::IdParseError;
pub use digest::{RevisionId, SnapshotId};
pub use entity::EntityId;
pub use names::RootName;
pub use operation::OperationId;
pub use project::ProjectId;
pub use type_id::TypeId;

/// Backward-compatible name for the canonical semantic type identifier.
pub type ObjectType = TypeId;
