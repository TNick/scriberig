//! Composes universal read and change-operation providers.

use crate::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, ApplyRequest, BranchRequest, CommitRequest, DiffRequest, HistoryRequest,
    MigrationDetail, OperationSummary, RenderRequest,
};
use scriberig_core::{RootName, SnapshotId};

/// Owns application-level diff, history, rendering, and change semantics.
pub trait OperationProvider {
    /// Reads canonical detail for one registered migration proposal.
    ///
    /// # Parameters
    /// - `self`: Canonical operation provider.
    /// - `_id`: Stable migration handle.
    ///
    /// # Returns
    /// Snapshot-bound migration detail.
    ///
    /// # Errors
    /// Returns not-found for an unknown migration or unsupported-operation when
    /// proposal reads are not configured.
    fn migration_detail(&self, _id: &str) -> Result<MigrationDetail, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "migration detail".to_owned(),
        })
    }
    /// Resolves a named root before a semantic operation starts.
    ///
    /// # Parameters
    /// - `self`: Canonical operation provider.
    /// - `_root`: Named root requested by the caller.
    ///
    /// # Returns
    /// One exact immutable snapshot.
    ///
    /// # Errors
    /// Returns unsupported-operation when root resolution is not configured.
    fn resolve_root(&self, _root: &RootName) -> Result<SnapshotId, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "named root resolution".to_owned(),
        })
    }

    /// Computes a semantic diff between two exact snapshots.
    ///
    /// # Parameters
    /// - `self`: Canonical operation provider.
    /// - `context`: Authorized application context.
    /// - `request`: Exact snapshots to compare.
    ///
    /// # Returns
    /// Structured diff result summary.
    ///
    /// # Errors
    /// Returns a structured operation failure.
    fn diff(
        &self,
        context: &ApplicationContext,
        request: &DiffRequest,
    ) -> Result<OperationSummary, ApplicationError>;

    /// Reads semantic history from one exact snapshot.
    ///
    /// # Parameters
    /// - `self`: Canonical operation provider.
    /// - `context`: Authorized application context.
    /// - `request`: History request with a resolved snapshot.
    ///
    /// # Returns
    /// Structured history result summary.
    ///
    /// # Errors
    /// Returns a structured operation failure.
    fn history(
        &self,
        context: &ApplicationContext,
        request: &HistoryRequest,
    ) -> Result<OperationSummary, ApplicationError>;

    /// Renders a semantic state through a registered projection.
    ///
    /// # Parameters
    /// - `self`: Canonical operation provider.
    /// - `context`: Authorized application context.
    /// - `request`: Render request with a resolved snapshot.
    ///
    /// # Returns
    /// Structured render result summary.
    ///
    /// # Errors
    /// Returns a structured operation failure.
    fn render(
        &self,
        context: &ApplicationContext,
        request: &RenderRequest,
    ) -> Result<OperationSummary, ApplicationError>;

    /// Creates a branch from an exact base snapshot.
    ///
    /// # Parameters
    /// - `self`: Canonical operation provider.
    /// - `context`: Authorized application context.
    /// - `request`: Branch request with a resolved base.
    ///
    /// # Returns
    /// Structured branch result summary.
    ///
    /// # Errors
    /// Returns a structured operation failure.
    fn branch(
        &self,
        context: &ApplicationContext,
        request: &BranchRequest,
    ) -> Result<OperationSummary, ApplicationError>;

    /// Applies a change without promoting a canonical root.
    ///
    /// # Parameters
    /// - `self`: Canonical operation provider.
    /// - `context`: Authorized application context.
    /// - `request`: Change and exact base snapshot.
    ///
    /// # Returns
    /// Structured candidate result summary.
    ///
    /// # Errors
    /// Returns validation or conflict details from the owning subsystem.
    fn apply(
        &self,
        context: &ApplicationContext,
        request: &ApplyRequest,
    ) -> Result<OperationSummary, ApplicationError>;

    /// Commits a validated change at an exact base snapshot.
    ///
    /// # Parameters
    /// - `self`: Canonical operation provider.
    /// - `context`: Authorized application context.
    /// - `request`: Change and exact base snapshot.
    ///
    /// # Returns
    /// Structured promoted-state result summary.
    ///
    /// # Errors
    /// Returns conflict, validation, or promotion failures.
    fn commit(
        &self,
        context: &ApplicationContext,
        request: &CommitRequest,
    ) -> Result<OperationSummary, ApplicationError>;
}

/// Composes an application read backend with canonical operation ownership.
pub struct ApplicationBackendComposition<R, O> {
    /// Backend owning inspection and other read operations.
    read: R,
    /// Provider owning diff, history, projection, and change operations.
    operations: O,
}

impl<R, O> ApplicationBackendComposition<R, O> {
    /// Creates one application backend from separate read and operation ports.
    ///
    /// # Parameters
    /// - `read`: Backend owning read operations.
    /// - `operations`: Provider owning non-read operations.
    ///
    /// # Returns
    /// A backend preserving the ownership of both providers.
    #[must_use]
    pub const fn new(read: R, operations: O) -> Self {
        Self { read, operations }
    }

    /// Returns the read backend.
    ///
    /// # Returns
    /// The backend used for read requests.
    #[must_use]
    pub const fn read(&self) -> &R {
        &self.read
    }

    /// Returns the operation provider.
    ///
    /// # Returns
    /// The provider used for change and projection requests.
    #[must_use]
    pub const fn operations(&self) -> &O {
        &self.operations
    }
}

impl<R, O> ApplicationBackend for ApplicationBackendComposition<R, O>
where
    R: ApplicationBackend,
    O: OperationProvider,
{
    /// Delegates each request to the provider owning its semantic operation.
    ///
    /// # Parameters
    /// - `context`: Authorized application context.
    /// - `request`: Universal application request.
    ///
    /// # Returns
    /// The structured result produced by the selected provider.
    ///
    /// # Errors
    /// Returns the selected provider's structured application error.
    fn execute(
        &self,
        context: &ApplicationContext,
        request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        match request {
            ApplicationRequest::Diff(value) => self
                .operations
                .diff(context, value)
                .map(ApplicationResult::Diff),
            ApplicationRequest::History(value) => {
                let value = resolve_history(&self.operations, value)?;
                self.operations
                    .history(context, &value)
                    .map(ApplicationResult::History)
            }
            ApplicationRequest::Render(value) => {
                let value = resolve_render(&self.operations, value)?;
                self.operations
                    .render(context, &value)
                    .map(ApplicationResult::Render)
            }
            ApplicationRequest::Branch(value) => {
                let value = resolve_branch(&self.operations, value)?;
                self.operations
                    .branch(context, &value)
                    .map(ApplicationResult::Branch)
            }
            ApplicationRequest::Apply(value) => self
                .operations
                .apply(context, value)
                .map(ApplicationResult::Apply),
            ApplicationRequest::Commit(value) => self
                .operations
                .commit(context, value)
                .map(ApplicationResult::Commit),
            _ => self.read.execute(context, request),
        }
    }

    /// Reads migration detail from the canonical operation provider.
    ///
    /// # Parameters
    /// - `_context`: Authorized application context.
    /// - `id`: Stable migration handle.
    ///
    /// # Returns
    /// Snapshot-bound migration detail.
    ///
    /// # Errors
    /// Returns the operation provider's structured application error.
    fn migration_detail(
        &self,
        _context: &ApplicationContext,
        id: &str,
    ) -> Result<MigrationDetail, ApplicationError> {
        self.operations.migration_detail(id)
    }
    /// Delegates current-snapshot resolution to the read backend.
    ///
    /// # Parameters
    /// - `context`: Authorized application context.
    ///
    /// # Returns
    /// The exact current snapshot.
    ///
    /// # Errors
    /// Returns the read backend's snapshot resolution error.
    fn current_snapshot(
        &self,
        context: &ApplicationContext,
    ) -> Result<SnapshotId, ApplicationError> {
        self.read.current_snapshot(context)
    }
}

/// Resolves a history selector exactly once before delegation.
fn resolve_history<O: OperationProvider>(
    provider: &O,
    request: &HistoryRequest,
) -> Result<HistoryRequest, ApplicationError> {
    Ok(HistoryRequest {
        snapshot: resolve_selector(provider, &request.snapshot)?,
    })
}

/// Resolves a render selector exactly once before delegation.
fn resolve_render<O: OperationProvider>(
    provider: &O,
    request: &RenderRequest,
) -> Result<RenderRequest, ApplicationError> {
    Ok(RenderRequest {
        snapshot: resolve_selector(provider, &request.snapshot)?,
        view: request.view.clone(),
    })
}

/// Resolves a branch selector exactly once before delegation.
fn resolve_branch<O: OperationProvider>(
    provider: &O,
    request: &BranchRequest,
) -> Result<BranchRequest, ApplicationError> {
    Ok(BranchRequest {
        base: resolve_selector(provider, &request.base)?,
        name: request.name.clone(),
    })
}

/// Converts a snapshot selector to one exact snapshot.
fn resolve_selector<O: OperationProvider>(
    provider: &O,
    selector: &crate::SnapshotSelector,
) -> Result<crate::SnapshotSelector, ApplicationError> {
    match selector {
        crate::SnapshotSelector::Exact(snapshot) => Ok(crate::SnapshotSelector::Exact(*snapshot)),
        crate::SnapshotSelector::NamedRoot(root) => {
            Ok(crate::SnapshotSelector::Exact(provider.resolve_root(root)?))
        }
    }
}
