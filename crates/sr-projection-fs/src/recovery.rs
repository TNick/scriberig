//! Defines retryable failed-import evidence.
use crate::{FileDigest, ProjectionLocation, ProjectionRef};
use sr_projection::{ProjectionDiagnostic, SemanticStateRef};
/// Stage at which an external import stopped.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FailureStage {
    Read,
    Parse,
    Reconcile,
    Validate,
    Commit,
    Rerender,
}
/// Immutable information required to retry a failed import.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryRecord {
    pub projection: ProjectionRef,
    pub base: SemanticStateRef,
    pub current: SemanticStateRef,
    pub external: FileDigest,
    pub location: ProjectionLocation,
    pub stage: FailureStage,
    pub diagnostics: Vec<ProjectionDiagnostic>,
    pub bytes: Vec<u8>,
}

/// Retains the latest failed import for each projection.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MemoryRecoveryStore {
    records: Vec<RecoveryRecord>,
}

impl MemoryRecoveryStore {
    /// Stores or replaces the failed import record for a projection.
    ///
    /// - `record`: Failed import evidence to retain.
    pub fn put(&mut self, record: RecoveryRecord) {
        if let Some(existing) = self
            .records
            .iter_mut()
            .find(|existing| existing.projection == record.projection)
        {
            *existing = record;
        } else {
            self.records.push(record);
        }
    }

    /// Finds retained failure evidence for a projection.
    ///
    /// - `projection`: Projection whose failure should be returned.
    ///
    /// The retained record when one exists.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn find(&self, projection: ProjectionRef) -> Option<&RecoveryRecord> {
        self.records
            .iter()
            .find(|record| record.projection == projection)
    }

    /// Removes retained failure evidence after a successful import.
    ///
    /// - `projection`: Projection whose failure should be removed.
    pub fn remove(&mut self, projection: ProjectionRef) {
        self.records
            .retain(|record| record.projection != projection);
    }
}
