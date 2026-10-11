//! Defines backend-neutral maintenance operations for immutable storage.

use crate::{IndexStatus, IntegrityReport, ProtectedRoots, StoreOperationError};

/// Provides integrity, reachability, garbage collection, and index rebuilds.
pub trait MaintenanceStore {
    /// Verifies canonical state without changing it.
    ///
    /// # Parameters
    /// - `protected`: Stable snapshot roots for this verification.
    ///
    /// # Returns
    /// Structured corruption and unreachable-object findings.
    ///
    /// # Errors
    /// Returns a backend maintenance error.
    fn integrity_check(
        &self,
        protected: &ProtectedRoots,
    ) -> Result<IntegrityReport, StoreOperationError>;

    /// Reports valid objects unreachable from protected roots.
    ///
    /// # Parameters
    /// - `protected`: Stable snapshot roots for this maintenance run.
    ///
    /// # Returns
    /// A dry-run integrity and reachability report.
    ///
    /// # Errors
    /// Returns a backend maintenance error.
    fn gc_dry_run(
        &self,
        protected: &ProtectedRoots,
    ) -> Result<IntegrityReport, StoreOperationError>;

    /// Sweeps valid objects unreachable from protected roots.
    ///
    /// # Parameters
    /// - `protected`: Stable snapshot roots for this maintenance run.
    ///
    /// # Returns
    /// The pre-sweep integrity and reachability report.
    ///
    /// # Errors
    /// Returns a backend maintenance error.
    fn gc_sweep(&self, protected: &ProtectedRoots) -> Result<IntegrityReport, StoreOperationError>;

    /// Rebuilds all backend-derived indexes from canonical data.
    ///
    /// # Returns
    /// The resulting index status.
    ///
    /// # Errors
    /// Returns a backend maintenance error.
    fn rebuild_indexes(&self) -> Result<IndexStatus, StoreOperationError>;
}
