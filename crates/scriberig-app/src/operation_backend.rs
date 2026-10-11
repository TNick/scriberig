//! Composes universal read and change-operation providers.

use crate::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, ApplyRequest, BranchRequest, CommitRequest, DiffRequest, HistoryRequest,
    OperationSummary, RenderRequest,
};
use scriberig_core::{RootName, SnapshotId};
use scriberig_query::{
    FindService, InspectService, QueryError, StructuredQueryService, TraverseService,
};

/// Owns application-level diff, history, rendering, and change semantics.
pub trait OperationProvider {
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

/// Composes a universal read provider with canonical operation ownership.
pub struct CompositeBackend<R, O> {
    read: R,
    operations: O,
    current_snapshot: Option<SnapshotId>,
}

impl<R, O> CompositeBackend<R, O> {
    /// Creates a composite backend from independent providers.
    ///
    /// # Parameters
    /// - `read`: Provider for inspect/list/follow/find/query.
    /// - `operations`: Provider for diff/history/render/branch/apply/commit.
    ///
    /// # Returns
    /// A universal application backend.
    #[must_use]
    pub const fn new(read: R, operations: O) -> Self {
        Self {
            read,
            operations,
            current_snapshot: None,
        }
    }

    /// Sets the snapshot used to resolve current-root selectors.
    ///
    /// # Parameters
    /// - `self`: Backend being configured.
    /// - `snapshot`: Exact current snapshot.
    ///
    /// # Returns
    /// The configured backend.
    #[must_use]
    pub const fn with_current_snapshot(mut self, snapshot: SnapshotId) -> Self {
        self.current_snapshot = Some(snapshot);
        self
    }
}

impl<R, O> ApplicationBackend for CompositeBackend<R, O>
where
    R: InspectService + TraverseService + FindService + StructuredQueryService,
    O: OperationProvider,
{
    /// Dispatches each universal operation to its owning provider.
    ///
    /// # Parameters
    /// - `self`: Composite application backend.
    /// - `context`: Authorized caller context.
    /// - `request`: Universal request to execute.
    ///
    /// # Returns
    /// Structured universal operation result.
    ///
    /// # Errors
    /// Returns the provider's structured application failure.
    fn execute(
        &self,
        context: &ApplicationContext,
        request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        match request {
            ApplicationRequest::Inspect(value) => self
                .read
                .inspect(value)
                .map(ApplicationResult::Inspect)
                .map_err(map_query_error),
            ApplicationRequest::List(value) => self
                .read
                .list(value)
                .map(ApplicationResult::List)
                .map_err(map_query_error),
            ApplicationRequest::Follow(value) => self
                .read
                .follow(value)
                .map(ApplicationResult::Follow)
                .map_err(map_query_error),
            ApplicationRequest::Find(value) => self
                .read
                .find(value)
                .map(ApplicationResult::Find)
                .map_err(map_query_error),
            ApplicationRequest::Query(value) => self
                .read
                .query(value)
                .map(ApplicationResult::Query)
                .map_err(map_query_error),
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
        }
    }

    /// Resolves the configured current snapshot for convenience APIs.
    ///
    /// # Parameters
    /// - `self`: Composite application backend.
    /// - `_context`: Authorized caller context.
    ///
    /// # Returns
    /// Exact configured current snapshot.
    ///
    /// # Errors
    /// Returns an unsupported-operation error when no root is configured.
    fn current_snapshot(
        &self,
        _context: &ApplicationContext,
    ) -> Result<SnapshotId, ApplicationError> {
        self.current_snapshot
            .ok_or_else(|| ApplicationError::UnsupportedOperation {
                operation: "current snapshot resolution".to_owned(),
            })
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

/// Maps a query error to the application boundary.
///
/// # Parameters
/// - `error`: Query failure returned by the read provider.
///
/// # Returns
/// Structured application-level failure.
fn map_query_error(error: QueryError) -> ApplicationError {
    match error {
        QueryError::NotFound { target, .. } => ApplicationError::NotFound {
            subject: format!("{target:?}"),
        },
        QueryError::SnapshotUnavailable { snapshot, .. } => {
            ApplicationError::SnapshotUnavailable { snapshot }
        }
        QueryError::UnsupportedCapability {
            operation,
            capability,
        } => ApplicationError::UnsupportedCapability {
            operation: format!("{operation:?}").to_lowercase(),
            capability,
        },
        QueryError::DerivedDataUnavailable { operation, .. } => {
            ApplicationError::UnsupportedCapability {
                operation: format!("{operation:?}").to_lowercase(),
                capability: "derived read data".to_owned(),
            }
        }
        QueryError::Internal { message, .. } => ApplicationError::Internal { message },
        QueryError::InvalidQuery { message } => ApplicationError::InvalidRequest {
            operation: "query".to_owned(),
            message,
        },
        _ => ApplicationError::InvalidRequest {
            operation: "universal read".to_owned(),
            message: "query request is invalid".to_owned(),
        },
    }
}
