//! Implements validated output writing and import completion for the coordinator.

use super::SyncCoordinator;
use crate::coordinator::ImportCompletion;
use crate::{FailureStage, FileSystem, IoError, RecoveryRecord, hash::digest_bytes};
use sr_projection::{ProjectionRef, ProjectionStatus, SemanticStateRef};

impl<F: FileSystem> SyncCoordinator<F> {
    /// Applies a domain import result without performing semantic mutation.
    ///
    /// - `projection`: Projection whose import completed.
    /// - `completion`: Domain result and exact bytes to synchronize.
    ///
    /// Unit after applying the completion.
    ///
    /// Returns an IO or ownership error when rerendering cannot complete.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn complete_import(
        &mut self,
        projection: ProjectionRef,
        completion: ImportCompletion,
    ) -> Result<(), IoError> {
        let result = self.complete_import_inner(projection, completion);
        self.states.persist()?;
        result
    }

    /// Applies an import result before the public method persists state.
    ///
    /// - `projection`: Projection whose import completed.
    /// - `completion`: Domain result and exact bytes to synchronize.
    ///
    /// # Returns
    /// Success after applying the import result.
    ///
    /// # Errors
    /// Returns an IO or ownership error when synchronization cannot complete.
    fn complete_import_inner(
        &mut self,
        projection: ProjectionRef,
        completion: ImportCompletion,
    ) -> Result<(), IoError> {
        match completion {
            ImportCompletion::NoSemanticChange {
                request,
                bytes,
                base,
                baseline,
            }
            | ImportCompletion::Committed {
                request,
                bytes,
                base,
                baseline,
            } => {
                if baseline.rendered.bytes != bytes || baseline.rendered.base_state != base {
                    return Err(IoError::BaselineMismatch);
                }
                self.write_rendered(projection, &bytes, base)?;
                let Some(state) = self.states.find_projection_mut(projection) else {
                    return Err(IoError::OwnershipDenied);
                };
                state.baseline = baseline;
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
                    base: request.baseline.rendered.base_state,
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
                    base: request.baseline.rendered.base_state,
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
    /// - `projection`: Projection whose output is being written.
    /// - `bytes`: Exact canonical bytes to write.
    /// - `base`: Semantic state represented by the written bytes.
    ///
    /// Unit after the exact bytes have been written and verified.
    ///
    /// Returns an IO or ownership error when writing or verification fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn write_rendered(
        &mut self,
        projection: ProjectionRef,
        bytes: &[u8],
        base: SemanticStateRef,
    ) -> Result<(), IoError> {
        let result = self.write_rendered_inner(projection, bytes, base);
        self.states.persist()?;
        result
    }

    /// Writes and verifies projection bytes before the public method persists state.
    ///
    /// - `projection`: Projection whose output is being written.
    /// - `bytes`: Exact output bytes.
    /// - `base`: Semantic state represented by the output.
    ///
    /// # Returns
    /// Success after the file and state agree.
    ///
    /// # Errors
    /// Returns an IO or ownership error when writing or verification fails.
    fn write_rendered_inner(
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
        if written != bytes {
            let Some(state) = self.states.find_projection_mut(projection) else {
                return Err(IoError::OwnershipDenied);
            };
            state.observed = Some(digest_bytes(&written));
            state.status = ProjectionStatus::ExternallyModified;
            return Err(IoError::BaselineMismatch);
        }
        let digest = digest_bytes(&written);
        let Some(state) = self.states.find_projection_mut(projection) else {
            return Err(IoError::OwnershipDenied);
        };
        state.baseline = state.baseline.with_output(written, base);
        state.current = base;
        state.observed = Some(digest);
        state.status = ProjectionStatus::Synchronized;
        self.recoveries.remove(projection);
        Ok(())
    }
}
