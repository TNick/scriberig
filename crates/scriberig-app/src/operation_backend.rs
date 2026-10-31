//! Composes universal read and change-operation providers.

use crate::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, ApplyRequest, BranchRequest, CommitRequest, DiffRequest, HistoryRequest,
    MergeRequest, MigrationDetail, OperationSummary, RenderRequest,
};
use scriberig_core::{RootName, SnapshotId};

/// Owns application-level diff, history, rendering, and change semantics.
pub trait OperationProvider {
    /// Reads canonical detail for one registered migration proposal.
    ///
    /// - `self`: Canonical operation provider.
    /// - `_id`: Stable migration handle.
    ///
    /// Snapshot-bound migration detail.
    ///
    /// Returns not-found for an unknown migration or unsupported-operation when
    /// proposal reads are not configured.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn migration_detail(&self, _id: &str) -> Result<MigrationDetail, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "migration detail".to_owned(),
        })
    }
    /// Resolves a named root before a semantic operation starts.
    ///
    /// - `self`: Canonical operation provider.
    /// - `_root`: Named root requested by the caller.
    ///
    /// One exact immutable snapshot.
    ///
    /// Returns unsupported-operation when root resolution is not configured.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn resolve_root(&self, _root: &RootName) -> Result<SnapshotId, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "named root resolution".to_owned(),
        })
    }

    /// Computes a semantic diff between two exact snapshots.
    ///
    /// - `self`: Canonical operation provider.
    /// - `context`: Authorized application context.
    /// - `request`: Exact snapshots to compare.
    ///
    /// Structured diff result summary.
    ///
    /// Returns a structured operation failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn diff(
        &self,
        context: &ApplicationContext,
        request: &DiffRequest,
    ) -> Result<OperationSummary, ApplicationError>;

    /// Merges exact candidate snapshots without promoting project state.
    ///
    /// - `self`: Canonical operation provider.
    /// - `context`: Authorized application context.
    /// - `request`: Exact common base and dependency candidates.
    ///
    /// Merge deltas, conflicts, and a candidate snapshot when conflict-free.
    ///
    /// Returns a structured operation failure.
    /// # Parameters
    /// - `_context`: Input for this operation.
    /// - `_request`: Input for this operation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn merge(
        &self,
        _context: &ApplicationContext,
        _request: &MergeRequest,
    ) -> Result<sr_change::MergeResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "candidate merge".to_owned(),
        })
    }

    /// Reads semantic history from one exact snapshot.
    ///
    /// - `self`: Canonical operation provider.
    /// - `context`: Authorized application context.
    /// - `request`: History request with a resolved snapshot.
    ///
    /// Structured history result summary.
    ///
    /// Returns a structured operation failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn history(
        &self,
        context: &ApplicationContext,
        request: &HistoryRequest,
    ) -> Result<OperationSummary, ApplicationError>;

    /// Renders a semantic state through a registered projection.
    ///
    /// - `self`: Canonical operation provider.
    /// - `context`: Authorized application context.
    /// - `request`: Render request with a resolved snapshot.
    ///
    /// Structured render result summary.
    ///
    /// Returns a structured operation failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn render(
        &self,
        context: &ApplicationContext,
        request: &RenderRequest,
    ) -> Result<OperationSummary, ApplicationError>;

    /// Creates a branch from an exact base snapshot.
    ///
    /// - `self`: Canonical operation provider.
    /// - `context`: Authorized application context.
    /// - `request`: Branch request with a resolved base.
    ///
    /// Structured branch result summary.
    ///
    /// Returns a structured operation failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn branch(
        &self,
        context: &ApplicationContext,
        request: &BranchRequest,
    ) -> Result<OperationSummary, ApplicationError>;

    /// Applies a change without promoting a canonical root.
    ///
    /// - `self`: Canonical operation provider.
    /// - `context`: Authorized application context.
    /// - `request`: Change and exact base snapshot.
    ///
    /// Structured candidate result summary.
    ///
    /// Returns validation or conflict details from the owning subsystem.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn apply(
        &self,
        context: &ApplicationContext,
        request: &ApplyRequest,
    ) -> Result<OperationSummary, ApplicationError>;

    /// Commits a validated change at an exact base snapshot.
    ///
    /// - `self`: Canonical operation provider.
    /// - `context`: Authorized application context.
    /// - `request`: Change and exact base snapshot.
    ///
    /// Structured promoted-state result summary.
    ///
    /// Returns conflict, validation, or promotion failures.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
    /// - `read`: Backend owning read operations.
    /// - `operations`: Provider owning non-read operations.
    ///
    /// A backend preserving the ownership of both providers.
    #[must_use]
    pub const fn new(read: R, operations: O) -> Self {
        Self { read, operations }
    }

    /// Returns the read backend.
    ///
    /// The backend used for read requests.
    #[must_use]
    pub const fn read(&self) -> &R {
        &self.read
    }

    /// Returns the operation provider.
    ///
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
    /// - `context`: Authorized application context.
    /// - `request`: Universal application request.
    ///
    /// The structured result produced by the selected provider.
    ///
    /// Returns the selected provider's structured application error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn execute(
        &self,
        context: &ApplicationContext,
        request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        match request {
            ApplicationRequest::Merge(value) => self
                .operations
                .merge(context, value)
                .map(ApplicationResult::Merge),
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
    /// - `_context`: Authorized application context.
    /// - `id`: Stable migration handle.
    ///
    /// Snapshot-bound migration detail.
    ///
    /// Returns the operation provider's structured application error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn migration_detail(
        &self,
        _context: &ApplicationContext,
        id: &str,
    ) -> Result<MigrationDetail, ApplicationError> {
        self.operations.migration_detail(id)
    }
    /// Delegates current-snapshot resolution to the read backend.
    ///
    /// - `context`: Authorized application context.
    ///
    /// The exact current snapshot.
    ///
    /// Returns the read backend's snapshot resolution error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
    if !request.style.is_applied() {
        // A provider holds no style configuration, so an unresolved selection
        // here would mean a surface invented a style of its own.
        return Err(ApplicationError::InvalidRequest {
            operation: "render".to_owned(),
            message: "the application must resolve the render style first".to_owned(),
        });
    }
    Ok(RenderRequest {
        snapshot: resolve_selector(provider, &request.snapshot)?,
        view: request.view.clone(),
        style: request.style.clone(),
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
