//! Defines backend-independent immutable revision, snapshot, and root storage contracts.

pub mod collection;
pub mod conformance;
pub mod integrity;
pub mod maintenance;
pub mod reachability;
pub mod revision;
pub mod revision_store;
pub mod roots;
pub mod snapshot;
pub mod snapshot_store;
pub mod stats;

pub use collection::Collection;
pub use conformance::{
    ConformanceFixture, ConformanceStore, FullConformanceStore, load_fixture, run_core_conformance,
    run_full_conformance,
};
pub use integrity::{IntegrityIssue, IntegrityReport};
pub use maintenance::MaintenanceStore;
pub use reachability::{ProtectedRoots, ReachableObjects, SnapshotNodeId, UnreachableObjects};
pub use revision::Revision;
pub use revision_store::{RevisionStore, StoreError};
pub use roots::{CasOutcome, RootError, RootStore};
pub use snapshot::{Snapshot, SnapshotChange, SnapshotError};
pub use snapshot_store::SnapshotStore;
pub use stats::{
    GcReport, IndexStatus, RootStatus, StoreErrorCategory, StoreObservability, StoreOperationError,
    StoreStats,
};
