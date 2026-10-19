//! Adapts the universal query providers to application execution.

use crate::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, CapabilityRegistry,
};
use scriberig_core::SnapshotId;
use scriberig_query::{
    FindService, InspectService, QueryError, StructuredQueryService, TraverseService,
};

/// Executes all implemented universal read operations through one provider.
pub struct UniversalReadBackend<R> {
    provider: R,
    current_snapshot: Option<SnapshotId>,
    capabilities: Option<CapabilityRegistry>,
}

impl<R> UniversalReadBackend<R> {
    /// Creates a read backend without an implicit current snapshot.
    ///
    /// - `provider`: Provider implementing the universal read contracts.
    ///
    /// A backend that requires exact snapshots for requests using `Current`.
    #[must_use]
    pub const fn new(provider: R) -> Self {
        Self {
            provider,
            current_snapshot: None,
            capabilities: None,
        }
    }

    /// Configures the snapshot used when a request selects `Current`.
    ///
    /// - `self`: Backend being configured.
    /// - `snapshot`: Exact snapshot representing the current root.
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
    /// The configured read backend.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_capability_registry(mut self, capabilities: CapabilityRegistry) -> Self {
        self.capabilities = Some(capabilities);
        self
    }

    /// Returns the underlying universal read provider.
    ///
    /// - `self`: Backend to inspect.
    ///
    /// The provider used for read execution.
    #[must_use]
    pub const fn provider(&self) -> &R {
        &self.provider
    }
}

impl<R> ApplicationBackend for UniversalReadBackend<R>
where
    R: InspectService + TraverseService + FindService + StructuredQueryService,
{
    /// Dispatches a universal read request to its typed provider contract.
    ///
    /// - `self`: Read backend.
    /// - `context`: Caller context, already authorized by the caller.
    /// - `request`: Universal request to execute.
    ///
    /// The typed universal read result.
    ///
    /// Returns a structured application error for provider or unsupported
    /// operation failures.
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
                let mut inspection = self.provider.inspect(value).map_err(map_query_error)?;
                if let Some(capabilities) = &self.capabilities {
                    capabilities.enrich_inspection(&mut inspection);
                }
                Ok(ApplicationResult::Inspect(inspection))
            }
            ApplicationRequest::List(value) => self
                .provider
                .list(value)
                .map(ApplicationResult::List)
                .map_err(map_query_error),
            ApplicationRequest::Follow(value) => self
                .provider
                .follow(value)
                .map(ApplicationResult::Follow)
                .map_err(map_query_error),
            ApplicationRequest::Find(value) => self
                .provider
                .find(value)
                .map(ApplicationResult::Find)
                .map_err(map_query_error),
            ApplicationRequest::Query(value) => self
                .provider
                .query(value)
                .map(ApplicationResult::Query)
                .map_err(map_query_error),
            ApplicationRequest::DomainOperation(value) => {
                let registry = self.capabilities.as_ref().ok_or_else(|| {
                    ApplicationError::UnsupportedOperation {
                        operation: "domain operation registry".to_owned(),
                    }
                })?;
                CapabilityRegistry::authorize(context, value)?;
                let inspection = self
                    .provider
                    .inspect(&crate::capability_registry::subject_inspection(value))
                    .map_err(map_query_error)?;
                registry
                    .execute_for_entity_type(context, value, inspection.entity_type.as_ref())
                    .map(ApplicationResult::DomainOperation)
            }
            _ => Err(ApplicationError::UnsupportedOperation {
                operation: "universal write or projection operation".to_owned(),
            }),
        }
    }

    /// Resolves the configured current snapshot.
    ///
    /// - `self`: Read backend.
    /// - `_context`: Caller context.
    ///
    /// The configured exact snapshot.
    ///
    /// Returns an unsupported-operation error when no current snapshot exists.
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
}

/// Maps query-provider failures to stable application categories.
///
/// - `error`: Query-provider failure to translate.
///
/// Application-level failure with a stable category.
/// # Returns
/// The operation result.
fn map_query_error(error: QueryError) -> ApplicationError {
    match error {
        QueryError::NotFound { target, .. } => ApplicationError::NotFound {
            subject: target_subject(target),
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
        QueryError::InvalidTarget { operation, .. }
        | QueryError::WrongType { operation, .. }
        | QueryError::InvalidContinuation { operation }
        | QueryError::InvalidCollection { operation, .. }
        | QueryError::InvalidRelation { operation, .. } => ApplicationError::InvalidRequest {
            operation: format!("{operation:?}").to_lowercase(),
            message: "query request is invalid".to_owned(),
        },
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
    }
}

/// Extracts a stable subject string from a query target.
///
/// - `target`: Query target whose entity identity is needed.
///
/// Stable textual entity identity.
/// # Returns
/// The operation result.
fn target_subject(target: scriberig_query::TargetRef) -> String {
    match target {
        scriberig_query::TargetRef::Snapshot(value) => value.entity.to_string(),
        scriberig_query::TargetRef::Revision(value) => value.entity.to_string(),
    }
}
