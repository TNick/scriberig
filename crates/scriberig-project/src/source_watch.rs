//! Coordinates exact project source observations with candidate writes.

use std::collections::HashMap;

use scriberig_core::{ProjectId, SnapshotId};

/// Classifies a watcher notification for a project source path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectSourceObservation {
    /// Exact bytes match a committed Scriberig candidate write.
    SelfWrite,
    /// The observation differs from the most recent committed candidate output.
    ExternalChange,
}

/// Reports watcher classification and any canonical snapshot imported for it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectSourceObservationResult {
    /// Whether the event matches candidate output or represents external input.
    pub observation: ProjectSourceObservation,
    /// Snapshot promoted after an external change, absent for self-writes.
    pub imported_snapshot: Option<SnapshotId>,
}

/// Tracks candidate output digests until project source watcher notifications arrive.
#[derive(Clone, Debug, Default)]
pub(crate) struct ProjectSourceWatchCoordinator {
    committed: HashMap<(ProjectId, String), Option<[u8; 32]>>,
}

impl ProjectSourceWatchCoordinator {
    /// Records one committed materialization outcome for an exact project path.
    ///
    /// - `project`: Project owning the source path.
    /// - `path`: Canonical project-relative source path.
    /// - `digest`: Exact output digest, or `None` for a committed deletion.
    pub(crate) fn record_committed(
        &mut self,
        project: ProjectId,
        path: String,
        digest: Option<[u8; 32]>,
    ) {
        self.committed.insert((project, path), digest);
    }

    /// Classifies observed bytes and clears stale suppression after divergence.
    ///
    /// Exact matching entries remain until a different observation replaces
    /// them, making duplicate watcher notifications idempotent.
    ///
    /// - `project`: Project owning the source path.
    /// - `path`: Project-relative source path from the watcher.
    /// - `observed`: Exact current digest, or `None` when the file is absent.
    ///
    /// # Returns
    /// Whether the event matches a committed candidate write.
    pub(crate) fn classify(
        &mut self,
        project: ProjectId,
        path: &str,
        observed: Option<[u8; 32]>,
    ) -> ProjectSourceObservation {
        let key = (project, path.to_owned());
        match self.committed.get(&key) {
            Some(expected) if *expected == observed => ProjectSourceObservation::SelfWrite,
            Some(_) => {
                self.committed.remove(&key);
                ProjectSourceObservation::ExternalChange
            }
            None => ProjectSourceObservation::ExternalChange,
        }
    }
}
