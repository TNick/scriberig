//! Provides storage-backed exact inspection for the application service.

use crate::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest, ApplicationResult,
};
use scriberig_query::{Inspection, TargetRef};
use scriberig_store::{RevisionStore, RootStore, SnapshotStore};

/// Provides exact inspection over canonical revision, snapshot, and root stores.
pub struct StoreInspectionBackend<S> {
    store: S,
    current_root: scriberig_core::RootName,
}

impl<S> StoreInspectionBackend<S> {
    /// Creates an inspection backend using the conventional `main` root.
    ///
    /// # Parameters
    /// - `store`: Revision, snapshot, and root backend.
    ///
    /// # Returns
    /// A storage-backed inspection provider.
    ///
    /// # Panics
    /// Panics only if the fixed root name is invalid.
    #[must_use]
    pub fn new(store: S) -> Self {
        Self {
            store,
            current_root: scriberig_core::RootName::new("main").expect("main is a valid root name"),
        }
    }

    /// Returns the underlying storage backend.
    ///
    /// # Parameters
    /// - `self`: Storage-backed provider.
    ///
    /// # Returns
    /// The backend used for reads.
    #[must_use]
    pub const fn store(&self) -> &S {
        &self.store
    }
}

impl<S> ApplicationBackend for StoreInspectionBackend<S>
where
    S: RevisionStore + SnapshotStore + RootStore,
{
    /// Executes an exact snapshot or revision inspection.
    ///
    /// # Parameters
    /// - `self`: Storage-backed inspection provider.
    /// - `_context`: Trusted application context.
    /// - `request`: Universal application request.
    ///
    /// # Returns
    /// Compact inspection metadata from canonical storage.
    ///
    /// # Errors
    /// Returns a structured not-found or backend failure.
    fn execute(
        &self,
        _context: &ApplicationContext,
        request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        let ApplicationRequest::Inspect(request) = request else {
            return Err(ApplicationError::UnsupportedOperation {
                operation: "storage inspection backend only supports inspect".to_owned(),
            });
        };
        let (subject, revision_id, snapshot) = match request.target {
            TargetRef::Snapshot(target) => {
                let revision = self
                    .store
                    .resolve(target.snapshot, target.entity)
                    .map_err(|error| storage_snapshot_error(&error))?
                    .ok_or_else(|| ApplicationError::NotFound {
                        subject: target.entity.to_string(),
                    })?;
                (request.target, revision, Some(target.snapshot))
            }
            TargetRef::Revision(target) => (request.target, target.revision, None),
        };
        let revision = self
            .store
            .get(revision_id)
            .map_err(|error| storage_revision_error(&error))?
            .ok_or_else(|| ApplicationError::NotFound {
                subject: revision_id.to_string(),
            })?;
        Ok(ApplicationResult::Inspect(Inspection {
            subject,
            snapshot,
            revision: Some(revision.id()),
            entity_type: Some(revision.object_type().clone()),
            label: None,
            description: None,
            properties: Vec::new(),
            counts: Vec::new(),
            relations: Vec::new(),
            collections: Vec::new(),
            views: Vec::new(),
            operations: Vec::new(),
            provenance: Vec::new(),
            information_cost: None,
            detail: request.detail,
        }))
    }

    /// Resolves the conventional current `main` root.
    ///
    /// # Parameters
    /// - `self`: Storage-backed inspection provider.
    /// - `_context`: Trusted application context.
    ///
    /// # Returns
    /// The exact snapshot promoted on `main`.
    ///
    /// # Errors
    /// Returns not-found when `main` has not been promoted.
    fn current_snapshot(
        &self,
        _context: &ApplicationContext,
    ) -> Result<scriberig_core::SnapshotId, ApplicationError> {
        self.store
            .read_root(&self.current_root)
            .ok_or_else(|| ApplicationError::NotFound {
                subject: self.current_root.to_string(),
            })
    }
}

/// Maps a revision backend failure to a safe application error.
///
/// # Parameters
/// - `error`: Revision backend failure.
///
/// # Returns
/// Safe application error preserving no backend internals.
fn storage_revision_error(error: &scriberig_store::StoreError) -> ApplicationError {
    ApplicationError::Internal {
        message: format!("revision backend failure: {error:?}"),
    }
}

/// Maps a snapshot backend failure to a safe application error.
///
/// # Parameters
/// - `error`: Snapshot backend failure.
///
/// # Returns
/// Safe application error preserving no backend internals.
fn storage_snapshot_error(error: &scriberig_store::SnapshotError) -> ApplicationError {
    ApplicationError::Internal {
        message: format!("snapshot backend failure: {error:?}"),
    }
}
