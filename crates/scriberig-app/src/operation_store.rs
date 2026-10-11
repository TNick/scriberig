//! Provides storage-backed semantic diff execution.

use crate::{
    ApplicationContext, ApplicationError, ApplyRequest, BranchRequest, CommitRequest, DiffRequest,
    HistoryRequest, OperationProvider, OperationSummary, RenderRequest,
};
use scriberig_store::SnapshotStore;
use sr_change::HistoryGraph;

/// Executes exact snapshot diffs through the canonical snapshot store.
pub struct StoreOperationProvider<S> {
    store: S,
    history: Option<HistoryGraph>,
}

impl<S> StoreOperationProvider<S> {
    /// Creates a storage-backed operation provider.
    ///
    /// # Parameters
    /// - `store`: Snapshot backend owning exact immutable states.
    ///
    /// # Returns
    /// A provider for storage-owned operations.
    #[must_use]
    pub const fn new(store: S) -> Self {
        Self {
            store,
            history: None,
        }
    }

    /// Installs an immutable history graph for history summaries.
    ///
    /// # Parameters
    /// - `self`: Provider being configured.
    /// - `history`: Canonical migration topology.
    ///
    /// # Returns
    /// Provider with history access enabled.
    #[must_use]
    pub fn with_history(mut self, history: HistoryGraph) -> Self {
        self.history = Some(history);
        self
    }

    /// Returns the underlying snapshot store.
    ///
    /// # Parameters
    /// - `self`: Provider to inspect.
    ///
    /// # Returns
    /// The canonical snapshot backend.
    #[must_use]
    pub const fn store(&self) -> &S {
        &self.store
    }
}

impl<S: SnapshotStore> OperationProvider for StoreOperationProvider<S> {
    /// Computes an exact snapshot diff through the store contract.
    ///
    /// # Parameters
    /// - `self`: Storage operation provider.
    /// - `_context`: Authorized application context.
    /// - `request`: Exact snapshots to compare.
    ///
    /// # Returns
    /// Summary containing the stable diff handle and right snapshot.
    ///
    /// # Errors
    /// Returns an internal application error when either snapshot is unavailable.
    fn diff(
        &self,
        _context: &ApplicationContext,
        request: &DiffRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        let changes = self
            .store
            .diff_snapshots(request.from, request.to)
            .map_err(|error| ApplicationError::SnapshotUnavailable {
                snapshot: match error {
                    scriberig_store::SnapshotError::UnknownSnapshot(snapshot) => snapshot,
                    scriberig_store::SnapshotError::Backend(_) => request.to,
                },
            })?;
        Ok(OperationSummary {
            handle: format!("diff:{}:{}:{}", request.from, request.to, changes.len()),
            snapshot: Some(request.to),
            revision: None,
        })
    }

    /// Reports that history storage is not configured by this provider.
    ///
    /// # Parameters
    /// - `self`: Storage operation provider.
    /// - `_context`: Authorized application context.
    /// - `request`: History request.
    ///
    /// # Returns
    /// Never returns a history summary.
    ///
    /// # Errors
    /// Returns an unsupported-operation error.
    fn history(
        &self,
        _context: &ApplicationContext,
        request: &HistoryRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        let Some(history) = &self.history else {
            return Err(unsupported("history"));
        };
        let snapshot = match request.snapshot {
            crate::SnapshotSelector::Exact(snapshot) => snapshot,
            crate::SnapshotSelector::NamedRoot(_) => {
                return Err(unsupported("history named-root resolution"));
            }
        };
        Ok(OperationSummary {
            handle: format!("history:{snapshot}:{}", history.migrations().len()),
            snapshot: Some(snapshot),
            revision: None,
        })
    }

    /// Reports that projection rendering is not owned by this provider.
    ///
    /// # Parameters
    /// - `self`: Storage operation provider.
    /// - `_context`: Authorized application context.
    /// - `_request`: Render request.
    ///
    /// # Returns
    /// Never returns a render summary.
    ///
    /// # Errors
    /// Returns an unsupported-operation error.
    fn render(
        &self,
        _context: &ApplicationContext,
        _request: &RenderRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        Err(unsupported("render"))
    }

    /// Reports that branch lifecycle storage is not configured.
    ///
    /// # Parameters
    /// - `self`: Storage operation provider.
    /// - `_context`: Authorized application context.
    /// - `_request`: Branch request.
    ///
    /// # Returns
    /// Never returns a branch summary.
    ///
    /// # Errors
    /// Returns an unsupported-operation error.
    fn branch(
        &self,
        _context: &ApplicationContext,
        _request: &BranchRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        Err(unsupported("branch"))
    }

    /// Reports that change application requires a configured migration service.
    ///
    /// # Parameters
    /// - `self`: Storage operation provider.
    /// - `_context`: Authorized application context.
    /// - `_request`: Apply request.
    ///
    /// # Returns
    /// Never returns a candidate summary.
    ///
    /// # Errors
    /// Returns an unsupported-operation error.
    fn apply(
        &self,
        _context: &ApplicationContext,
        _request: &ApplyRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        Err(unsupported("apply"))
    }

    /// Reports that commit requires a configured promotion service.
    ///
    /// # Parameters
    /// - `self`: Storage operation provider.
    /// - `_context`: Authorized application context.
    /// - `_request`: Commit request.
    ///
    /// # Returns
    /// Never returns a promotion summary.
    ///
    /// # Errors
    /// Returns an unsupported-operation error.
    fn commit(
        &self,
        _context: &ApplicationContext,
        _request: &CommitRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        Err(unsupported("commit"))
    }
}

/// Creates a stable unsupported-operation error.
///
/// # Parameters
/// - `operation`: Operation name that lacks a configured provider.
///
/// # Returns
/// Structured unsupported-operation failure.
fn unsupported(operation: &str) -> ApplicationError {
    ApplicationError::UnsupportedOperation {
        operation: operation.to_owned(),
    }
}
