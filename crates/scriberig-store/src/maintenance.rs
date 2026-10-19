//! Defines backend-neutral maintenance operations for immutable storage.

use crate::{IndexStatus, IntegrityReport, ProtectedRoots, StoreOperationError};

/// Provides integrity, reachability, garbage collection, and index rebuilds.
pub trait MaintenanceStore {
    /// Verifies canonical state without changing it.
    ///
    /// - `protected`: Stable snapshot roots for this verification.
    ///
    /// Structured corruption and unreachable-object findings.
    ///
    /// Returns a backend maintenance error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn integrity_check(
        &self,
        protected: &ProtectedRoots,
    ) -> Result<IntegrityReport, StoreOperationError>;

    /// Reports valid objects unreachable from protected roots.
    ///
    /// - `protected`: Stable snapshot roots for this maintenance run.
    ///
    /// A dry-run integrity and reachability report.
    ///
    /// Returns a backend maintenance error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn gc_dry_run(
        &self,
        protected: &ProtectedRoots,
    ) -> Result<IntegrityReport, StoreOperationError>;

    /// Sweeps valid objects unreachable from protected roots.
    ///
    /// - `protected`: Stable snapshot roots for this maintenance run.
    ///
    /// The pre-sweep integrity and reachability report.
    ///
    /// Returns a backend maintenance error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn gc_sweep(&self, protected: &ProtectedRoots) -> Result<IntegrityReport, StoreOperationError>;

    /// Rebuilds all backend-derived indexes from canonical data.
    ///
    /// The resulting index status.
    ///
    /// Returns a backend maintenance error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn rebuild_indexes(&self) -> Result<IndexStatus, StoreOperationError>;
}
