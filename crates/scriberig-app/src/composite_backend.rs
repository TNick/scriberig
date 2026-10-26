//! Provides the legacy composite backend for universal application operations.

use crate::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, BranchRequest, CapabilityRegistry, HistoryRequest, OperationProvider,
    RenderRequest, SnapshotSelector,
};
use scriberig_core::SnapshotId;
use scriberig_query::{
    FindService, InspectService, QueryError, StructuredQueryService, TraverseService,
};

/// Composes a universal read provider with canonical operation ownership.
pub struct CompositeBackend<R, O> {
    read: R,
    operations: O,
    current_snapshot: Option<SnapshotId>,
    capabilities: Option<CapabilityRegistry>,
}

impl<R, O> CompositeBackend<R, O> {
    /// Creates a composite backend from independent providers.
    ///
    /// - `read`: Provider for inspect/list/follow/find/query.
    /// - `operations`: Provider for diff/history/render/branch/apply/commit.
    ///
    /// A universal application backend.
    #[must_use]
    pub const fn new(read: R, operations: O) -> Self {
        Self {
            read,
            operations,
            current_snapshot: None,
            capabilities: None,
        }
    }

    /// Sets the snapshot used to resolve current-root selectors.
    ///
    /// - `self`: Backend being configured.
    /// - `snapshot`: Exact current snapshot.
    ///
    /// The configured backend.
    #[must_use]
    pub const fn with_current_snapshot(mut self, snapshot: SnapshotId) -> Self {
        self.current_snapshot = Some(snapshot);
        self
    }

    /// Installs the registry used to enrich inspection capabilities.
    ///
    /// - `self`: Backend being configured.
    /// - `capabilities`: Injected generic/domain capability registry.
    ///
    /// The configured composite backend.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_capability_registry(mut self, capabilities: CapabilityRegistry) -> Self {
        self.capabilities = Some(capabilities);
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
    /// - `self`: Composite application backend.
    /// - `context`: Authorized caller context.
    /// - `request`: Universal request to execute.
    ///
    /// Structured universal operation result.
    ///
    /// Returns the provider's structured application failure.
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
            ApplicationRequest::Inspect(value) => {
                let mut inspection = self.read.inspect(value).map_err(map_query_error)?;
                if let Some(capabilities) = &self.capabilities {
                    capabilities.enrich_inspection(&mut inspection);
                }
                Ok(ApplicationResult::Inspect(inspection))
            }
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
            ApplicationRequest::Merge(value) => self
                .operations
                .merge(context, value)
                .map(ApplicationResult::Merge),
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
            ApplicationRequest::DomainOperation(value) => {
                let registry = self.capabilities.as_ref().ok_or_else(|| {
                    ApplicationError::UnsupportedOperation {
                        operation: "domain operation registry".to_owned(),
                    }
                })?;
                CapabilityRegistry::authorize(context, value)?;
                let inspection = self
                    .read
                    .inspect(&crate::capability_registry::subject_inspection(value))
                    .map_err(map_query_error)?;
                registry
                    .execute_for_entity_type(context, value, inspection.entity_type.as_ref())
                    .map(ApplicationResult::DomainOperation)
            }
        }
    }

    /// Resolves the configured current snapshot for convenience APIs.
    ///
    /// - `self`: Composite application backend.
    /// - `_context`: Authorized caller context.
    ///
    /// Exact configured current snapshot.
    ///
    /// Returns an unsupported-operation error when no root is configured.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn current_snapshot(
        &self,
        _context: &ApplicationContext,
    ) -> Result<SnapshotId, ApplicationError> {
        self.current_snapshot
            .ok_or_else(|| ApplicationError::UnsupportedOperation {
                operation: "current snapshot resolution".to_owned(),
            })
    }

    /// Dispatches cancellable inspection through the composed read provider.
    ///
    /// - `self`: Composite application backend.
    /// - `context`: Authorized caller context.
    /// - `request`: Universal application request.
    /// - `cancelled`: Caller-owned cancellation flag.
    ///
    /// Structured application result when the operation completes.
    ///
    /// # Returns
    /// The operation result.
    ///
    /// # Errors
    /// Returns cancellation or a structured provider failure.
    fn execute_cancellable(
        &self,
        context: &ApplicationContext,
        request: &ApplicationRequest,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<ApplicationResult, ApplicationError> {
        if let ApplicationRequest::Inspect(value) = request {
            let mut inspection = self
                .read
                .inspect_cancellable(value, cancelled)
                .map_err(map_query_error)?;
            if cancelled.load(std::sync::atomic::Ordering::Acquire) {
                return Err(ApplicationError::OperationCancelled {
                    operation: "inspect".to_owned(),
                });
            }
            if let Some(capabilities) = &self.capabilities {
                capabilities.enrich_inspection(&mut inspection);
            }
            return Ok(ApplicationResult::Inspect(inspection));
        }
        self.execute(context, request)
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
    selector: &SnapshotSelector,
) -> Result<SnapshotSelector, ApplicationError> {
    match selector {
        SnapshotSelector::Exact(snapshot) => Ok(SnapshotSelector::Exact(*snapshot)),
        SnapshotSelector::NamedRoot(root) => {
            Ok(SnapshotSelector::Exact(provider.resolve_root(root)?))
        }
    }
}

/// Maps a query error to the application boundary.
///
/// - `error`: Query failure returned by the read provider.
///
/// Structured application-level failure.
/// # Returns
/// The operation result.
fn map_query_error(error: QueryError) -> ApplicationError {
    match error {
        QueryError::Cancelled { operation } => ApplicationError::OperationCancelled {
            operation: format!("{operation:?}").to_lowercase(),
        },
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
        QueryError::InvalidScope { operation, scope } => ApplicationError::InvalidRequest {
            operation: format!("{operation:?}").to_lowercase(),
            message: format!("invalid query scope: {scope:?}"),
        },
        QueryError::Ambiguous { operation, message } => ApplicationError::Ambiguous {
            scope: format!("{operation:?}").to_lowercase(),
            candidates: Vec::new(),
            reason: message,
        },
        _ => ApplicationError::InvalidRequest {
            operation: "universal read".to_owned(),
            message: "query request is invalid".to_owned(),
        },
    }
}
