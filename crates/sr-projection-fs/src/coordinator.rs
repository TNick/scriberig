//! Coordinates filesystem observation, import scheduling, and resynchronization.

use super::{ImportCompletion, ImportRequest, MemorySyncStore, ObservationResult};
use crate::{
    FailureStage, FileSystem, IoError, MemoryRecoveryStore, Observation, RecoveryRecord,
    hash::digest_bytes,
};
use sr_projection::{ProjectionRef, ProjectionStatus, SemanticStateRef};

/// Owns the filesystem boundary while leaving semantic interpretation to adapters.
#[derive(Clone, Debug)]
pub struct SyncCoordinator<F> {
    filesystem: F,
    states: MemorySyncStore,
    recoveries: MemoryRecoveryStore,
}

impl<F: FileSystem> SyncCoordinator<F> {
    /// Creates a coordinator with existing synchronization records.
    ///
    /// # Parameters
    /// - `filesystem`: Filesystem implementation.
    /// - `states`: Initial synchronization state records.
    ///
    /// # Returns
    /// A coordinator ready to observe and resynchronize projections.
    #[must_use]
    pub fn new(filesystem: F, states: MemorySyncStore) -> Self {
        Self {
            filesystem,
            states,
            recoveries: MemoryRecoveryStore::default(),
        }
    }

    /// Borrows current synchronization records.
    ///
    /// # Returns
    /// The current synchronization state store.
    #[must_use]
    pub fn states(&self) -> &MemorySyncStore {
        &self.states
    }

    /// Borrows mutable synchronization records.
    ///
    /// # Returns
    /// The mutable synchronization state store.
    pub fn states_mut(&mut self) -> &mut MemorySyncStore {
        &mut self.states
    }

    /// Borrows retained failed-import evidence.
    ///
    /// # Returns
    /// The retained recovery store.
    #[must_use]
    pub fn recoveries(&self) -> &MemoryRecoveryStore {
        &self.recoveries
    }

    /// Reconstructs the retained import request for a failed projection.
    ///
    /// # Parameters
    /// - `projection`: Projection whose failed import should be retried.
    ///
    /// # Returns
    /// The exact retained import request when both state and recovery evidence exist.
    #[must_use]
    pub fn retry_request(&self, projection: ProjectionRef) -> Option<ImportRequest> {
        let state = self.states.find_projection(projection)?;
        let recovery = self.recoveries.find(projection)?;
        Some(ImportRequest {
            projection,
            specification: state.specification,
            scope: state.scope,
            kind: state.kind.clone(),
            base: recovery.base,
            current: recovery.current,
            bytes: recovery.bytes.clone(),
            digest: recovery.external,
            location: recovery.location.clone(),
            style: state.style.clone(),
            renderer: state.renderer.clone(),
        })
    }

    /// Inspects a watcher hint and schedules domain import when required.
    ///
    /// # Parameters
    /// - `event`: Watcher-neutral observation hint.
    ///
    /// # Returns
    /// The resulting state and optional exact import request.
    ///
    /// # Errors
    /// Returns an IO error when current file contents cannot be inspected.
    pub fn observe(&mut self, event: &Observation) -> Result<Option<ObservationResult>, IoError> {
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
        if digest == state.rendered {
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
                specification: state.specification,
                scope: state.scope,
                kind: state.kind.clone(),
                base: state.base,
                current: state.current,
                bytes,
                digest,
                location: state.location.clone(),
                style: state.style.clone(),
                renderer: state.renderer.clone(),
            })
        } else {
            None
        };
        Ok(Some(ObservationResult {
            status: state.status,
            import,
        }))
    }

    /// Applies a domain import result without performing semantic mutation.
    ///
    /// # Parameters
    /// - `projection`: Projection whose import completed.
    /// - `completion`: Domain result and exact bytes to synchronize.
    ///
    /// # Returns
    /// Unit after applying the completion.
    ///
    /// # Errors
    /// Returns an IO or ownership error when rerendering cannot complete.
    pub fn complete_import(
        &mut self,
        projection: ProjectionRef,
        completion: ImportCompletion,
    ) -> Result<(), IoError> {
        match completion {
            ImportCompletion::NoSemanticChange {
                request,
                bytes,
                base,
            }
            | ImportCompletion::Committed {
                request,
                bytes,
                base,
            } => {
                self.write_rendered(projection, &bytes, base)?;
                self.recoveries.remove(request.projection);
                Ok(())
            }
            ImportCompletion::Failed {
                request,
                stage,
                diagnostic,
            } => {
                let Some(state) = self.states.find_projection_mut(projection) else {
                    return Err(IoError::OwnershipDenied);
                };
                state.status = ProjectionStatus::ImportFailed;
                self.recoveries.put(RecoveryRecord {
                    projection: request.projection,
                    base: request.base,
                    current: request.current,
                    external: request.digest,
                    location: request.location,
                    stage,
                    diagnostics: vec![diagnostic],
                    bytes: request.bytes,
                });
                Ok(())
            }
            ImportCompletion::Conflict {
                request,
                diagnostic,
            } => {
                let Some(state) = self.states.find_projection_mut(projection) else {
                    return Err(IoError::OwnershipDenied);
                };
                state.status = ProjectionStatus::Conflict;
                self.recoveries.put(RecoveryRecord {
                    projection: request.projection,
                    base: request.base,
                    current: request.current,
                    external: request.digest,
                    location: request.location,
                    stage: FailureStage::Reconcile,
                    diagnostics: vec![diagnostic],
                    bytes: request.bytes,
                });
                Ok(())
            }
        }
    }

    /// Atomically writes canonical bytes and establishes a synchronization base.
    ///
    /// # Parameters
    /// - `projection`: Projection whose output is being written.
    /// - `bytes`: Exact canonical bytes to write.
    /// - `base`: Semantic state represented by the written bytes.
    ///
    /// # Returns
    /// Unit after the exact bytes have been written and verified.
    ///
    /// # Errors
    /// Returns an IO or ownership error when writing or verification fails.
    pub fn write_rendered(
        &mut self,
        projection: ProjectionRef,
        bytes: &[u8],
        base: SemanticStateRef,
    ) -> Result<(), IoError> {
        let (location, previous_status) = {
            let Some(state) = self.states.find_projection_mut(projection) else {
                return Err(IoError::OwnershipDenied);
            };
            if !state.ownership.may_write() {
                return Err(IoError::OwnershipDenied);
            }
            let previous_status = state.status;
            state.status = ProjectionStatus::Rendering;
            (state.location.clone(), previous_status)
        };
        if let Err(error) = self.filesystem.write_atomic(location.path(), bytes) {
            if let Some(state) = self.states.find_projection_mut(projection) {
                state.status = previous_status;
            }
            return Err(error);
        }
        let written = match self.filesystem.read(location.path()) {
            Ok(written) => written,
            Err(error) => {
                if let Some(state) = self.states.find_projection_mut(projection) {
                    state.status = previous_status;
                }
                return Err(error);
            }
        };
        let digest = digest_bytes(&written);
        let Some(state) = self.states.find_projection_mut(projection) else {
            return Err(IoError::OwnershipDenied);
        };
        state.base = base;
        state.current = base;
        state.rendered = digest;
        state.observed = Some(digest);
        state.status = ProjectionStatus::Synchronized;
        self.recoveries.remove(projection);
        Ok(())
    }
}

/// Inspects one observation through the compatibility status-only API.
///
/// # Parameters
/// - `fs`: Filesystem implementation.
/// - `store`: Synchronization state store.
/// - `event`: Watcher-neutral observation hint.
///
/// # Returns
/// The resulting projection status when the location is known.
///
/// # Errors
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
