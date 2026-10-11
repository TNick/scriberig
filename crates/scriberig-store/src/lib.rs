//! Defines backend-independent immutable revision, snapshot, and root storage contracts.

pub mod collection;
pub mod conformance;
pub mod integrity;
pub mod reachability;
pub mod revision;
pub mod revision_store;
pub mod roots;
pub mod snapshot;
pub mod stats;

pub use collection::Collection;
pub use conformance::{ConformanceStore, run_core_conformance};
pub use integrity::{IntegrityIssue, IntegrityReport};
pub use reachability::{ProtectedRoots, ReachableObjects, UnreachableObjects};
pub use revision::Revision;
pub use revision_store::{RevisionStore, StoreError};
pub use roots::{CasOutcome, RootError, RootStore};
pub use snapshot::{Snapshot, SnapshotChange, SnapshotError, SnapshotStore};
pub use stats::{
    GcReport, IndexStatus, RootStatus, StoreErrorCategory, StoreObservability, StoreStats,
};
