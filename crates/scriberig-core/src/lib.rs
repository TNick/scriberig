//! Provides universal identifiers and domain-neutral core values for Scriberig.

// Expose canonical encoding and identity primitives to consumers.
/// Defines deterministic canonical byte encoding primitives.
pub mod encoding;
/// Defines strongly typed universal identifiers and names.
pub mod ids;

// Re-export the stable identifier API at the crate root.
pub use encoding::{CanonicalEncode, CanonicalWriter, EncodingVersion};
pub use ids::{EntityId, IdParseError, ObjectType, OperationId, RevisionId, RootName, SnapshotId};
