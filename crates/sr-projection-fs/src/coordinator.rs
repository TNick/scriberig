//! Coordinates filesystem observation, import scheduling, and resynchronization.

use super::{ImportCompletion, ImportRequest, MemorySyncStore, ObservationResult};
use crate::{
    ArtifactMaterializationPlan, ArtifactMaterializationReceipt, FileSystem, IoError,
    MemoryRecoveryStore, Observation, hash::digest_bytes,
};
use sr_projection::{ProjectionRef, ProjectionStatus};

mod materialization;

/// Owns the filesystem boundary while leaving semantic interpretation to adapters.
#[derive(Clone, Debug)]
pub struct SyncCoordinator<F> {
    filesystem: F,
    states: MemorySyncStore,
    recoveries: MemoryRecoveryStore,
}

impl<F: FileSystem> SyncCoordinator<F> {
    /// Records only committed artifact members for exact watcher self-write suppression.
    ///
    /// Uncommitted, failed, missing, or unknown members do not alter observation state.
    /// The receipt and plan must identify the same artifact.
    ///
    /// # Parameters
    /// - `plan`: Exact materialization plan mapping member keys to project-relative paths.
    /// - `receipt`: Per-member outcomes from that materialization attempt.
    ///
    /// # Returns
    /// Number of committed members matched to owned synchronization records.
    ///
    /// # Errors
    /// Returns an IO error when persistent synchronization state cannot be saved.
    pub fn record_artifact_materialization(
        &mut self,
        plan: &ArtifactMaterializationPlan,
        receipt: &ArtifactMaterializationReceipt,
    ) -> Result<usize, IoError> {
        if plan.artifact != receipt.artifact {
            return Ok(0);
        }
        let mut recorded = 0;
        for outcome in receipt.members.iter().filter(|member| member.committed) {
            let Some(member) = plan.members.iter().find(|member| member.key == outcome.key) else {
                continue;
            };
            if member.bytes.as_deref().map(crate::digest_bytes) != outcome.output_digest {
                continue;
            }
            let location = crate::ProjectionLocation::new(plan.root.join(&member.relative_path));
            let Some(state) = self.states.find_mut(&location) else {
                continue;
            };
            if member.bytes.is_some() {
                state.self_write_digest = outcome.output_digest;
            } else {
                state.self_write_digest = None;
                state.observed = None;
                state.status = ProjectionStatus::Missing;
            }
            recorded += 1;
        }
        self.states.persist()?;
        Ok(recorded)
    }

    /// Creates a coordinator with existing synchronization records.
    ///
    /// - `filesystem`: Filesystem implementation.
    /// - `states`: Initial synchronization state records.
    ///
    /// A coordinator ready to observe and resynchronize projections.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn new(filesystem: F, states: MemorySyncStore) -> Self {
        Self {
            filesystem,
            states,
            recoveries: MemoryRecoveryStore::default(),
        }
    }

    /// Opens a coordinator with synchronization baselines retained across restarts.
    ///
    /// - `filesystem`: Filesystem boundary used for projections.
    /// - `path`: Atomic persistence file for synchronization records.
    ///
    /// # Returns
    /// A coordinator loaded with previously synchronized baselines.
    ///
    /// # Errors
    /// Returns an IO error when persisted state cannot be read or decoded.
    pub fn open_persistent(
        filesystem: F,
        path: impl Into<std::path::PathBuf>,
    ) -> Result<Self, IoError> {
        Ok(Self::new(filesystem, MemorySyncStore::open(path)?))
    }

    /// Registers a projection state and persists it when the store is durable.
    ///
    /// - `state`: Exact projection baseline and ownership state to track.
    ///
    /// # Returns
    /// Success after the state is retained.
    ///
    /// # Errors
    /// Returns an IO error when durable state cannot be written.
    pub fn register_state(&mut self, state: super::SyncState) -> Result<(), IoError> {
        self.states.put(state);
        self.states.persist()
    }

    /// Borrows current synchronization records.
    ///
    /// The current synchronization state store.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn states(&self) -> &MemorySyncStore {
        &self.states
    }

    /// Borrows mutable synchronization records.
    ///
    /// Direct changes to a file-backed store must be followed by
    /// [`MemorySyncStore::persist`]. Prefer [`Self::register_state`] when adding
    /// a projection so the write result is propagated.
    ///
    /// The mutable synchronization state store.
    /// # Returns
    /// The operation result.
    pub fn states_mut(&mut self) -> &mut MemorySyncStore {
        &mut self.states
    }

    /// Borrows retained failed-import evidence.
    ///
    /// The retained recovery store.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn recoveries(&self) -> &MemoryRecoveryStore {
        &self.recoveries
    }

    /// Reconstructs the retained import request for a failed projection.
    ///
    /// - `projection`: Projection whose failed import should be retried.
    ///
    /// The exact retained import request when both state and recovery evidence exist.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn retry_request(&self, projection: ProjectionRef) -> Option<ImportRequest> {
        let state = self.states.find_projection(projection)?;
        let recovery = self.recoveries.find(projection)?;
        Some(ImportRequest {
            projection,
            baseline: state.baseline.clone(),
            current: recovery.current,
            bytes: recovery.bytes.clone(),
            digest: recovery.external,
            location: recovery.location.clone(),
        })
    }

    /// Inspects a watcher hint and schedules domain import when required.
    ///
    /// - `event`: Watcher-neutral observation hint.
    ///
    /// The resulting state and optional exact import request.
    ///
    /// Returns an IO error when current file contents cannot be inspected.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn observe(&mut self, event: &Observation) -> Result<Option<ObservationResult>, IoError> {
        let result = self.observe_inner(event);
        self.states.persist()?;
        result
    }

    /// Applies a watcher hint before the public method persists state.
    ///
    /// - `event`: Watcher-neutral observation hint.
    ///
    /// # Returns
    /// The resulting state and optional exact import request.
    ///
    /// # Errors
    /// Returns a filesystem inspection error.
    fn observe_inner(&mut self, event: &Observation) -> Result<Option<ObservationResult>, IoError> {
        let location = match event {
            Observation::Created(location)
            | Observation::Modified(location)
            | Observation::Removed(location)
            | Observation::RescanRequired(location) => location.clone(),
            Observation::Renamed { from, to } => {
                if !self.states.rename(from, to.clone()) {
                    return Ok(None);
                }
                to.clone()
            }
        };
        if matches!(event, Observation::Removed(_)) {
            let Some(state) = self.states.find_mut(&location) else {
                return Ok(None);
            };
            state.status = ProjectionStatus::Missing;
            return Ok(Some(ObservationResult {
                status: state.status,
                import: None,
            }));
        }
        let Some(_metadata) = self.filesystem.metadata(location.path())? else {
            let Some(state) = self.states.find_mut(&location) else {
                return Ok(None);
            };
            state.status = ProjectionStatus::Missing;
            return Ok(Some(ObservationResult {
                status: state.status,
                import: None,
            }));
        };
        let bytes = self.filesystem.read(location.path())?;
        let digest = digest_bytes(&bytes);
        let Some(state) = self.states.find_mut(&location) else {
            return Ok(None);
        };
        state.observed = Some(digest);
        if state.self_write_digest == Some(digest) {
            return Ok(Some(ObservationResult {
                status: state.status,
                import: None,
            }));
        }
        state.self_write_digest = None;
        if digest == crate::digest_bytes(&state.baseline.rendered.bytes) {
            state.status = ProjectionStatus::Synchronized;
            return Ok(Some(ObservationResult {
                status: state.status,
                import: None,
            }));
        }
        state.status = ProjectionStatus::ExternallyModified;
        let import = if state.ownership.may_import() {
            state.status = ProjectionStatus::ImportPending;
            Some(ImportRequest {
                projection: state.projection,
                baseline: state.baseline.clone(),
                current: state.current,
                bytes,
                digest,
                location: state.location.clone(),
            })
        } else {
            None
        };
        Ok(Some(ObservationResult {
            status: state.status,
            import,
        }))
    }
}

/// Inspects one observation through the compatibility status-only API.
///
/// - `fs`: Filesystem implementation.
/// - `store`: Synchronization state store.
/// - `event`: Watcher-neutral observation hint.
///
/// The resulting projection status when the location is known.
///
/// Returns a filesystem read error.
pub fn inspect_observation<F: FileSystem>(
    fs: &F,
    store: &mut MemorySyncStore,
    event: &Observation,
) -> Result<Option<ProjectionStatus>, IoError> {
    let mut coordinator = SyncCoordinator::new(fs, std::mem::take(store));
    let result = coordinator.observe(event);
    *store = coordinator.states;
    result.map(|result| result.map(|result| result.status))
}
