//! Adapts the universal query providers to application execution.

use crate::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest, ApplicationResult,
};
use scriberig_core::SnapshotId;
use scriberig_query::{
    FindService, InspectService, QueryError, StructuredQueryService, TraverseService,
};

/// Executes all implemented universal read operations through one provider.
pub struct UniversalReadBackend<R> {
    provider: R,
    current_snapshot: Option<SnapshotId>,
}

impl<R> UniversalReadBackend<R> {
    /// Creates a read backend without an implicit current snapshot.
    ///
    /// # Parameters
    /// - `provider`: Provider implementing the universal read contracts.
    ///
    /// # Returns
    /// A backend that requires exact snapshots for requests using `Current`.
    #[must_use]
    pub const fn new(provider: R) -> Self {
        Self {
            provider,
            current_snapshot: None,
        }
    }

    /// Configures the snapshot used when a request selects `Current`.
    ///
    /// # Parameters
    /// - `self`: Backend being configured.
    /// - `snapshot`: Exact snapshot representing the current root.
    ///
    /// # Returns
    /// The configured backend.
    #[must_use]
    pub const fn with_current_snapshot(mut self, snapshot: SnapshotId) -> Self {
        self.current_snapshot = Some(snapshot);
        self
    }

    /// Returns the underlying universal read provider.
    ///
    /// # Parameters
    /// - `self`: Backend to inspect.
    ///
    /// # Returns
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
    /// # Parameters
    /// - `self`: Read backend.
    /// - `_context`: Caller context, already authorized by the caller.
    /// - `request`: Universal request to execute.
    ///
    /// # Returns
    /// The typed universal read result.
    ///
    /// # Errors
    /// Returns a structured application error for provider or unsupported
    /// operation failures.
    fn execute(
        &self,
        _context: &ApplicationContext,
        request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        match request {
            ApplicationRequest::Inspect(value) => self
                .provider
                .inspect(value)
                .map(ApplicationResult::Inspect)
                .map_err(map_query_error),
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
            _ => Err(ApplicationError::UnsupportedOperation {
                operation: "universal write or projection operation".to_owned(),
            }),
        }
    }

    /// Resolves the configured current snapshot.
    ///
    /// # Parameters
    /// - `self`: Read backend.
    /// - `_context`: Caller context.
    ///
    /// # Returns
    /// The configured exact snapshot.
    ///
    /// # Errors
    /// Returns an unsupported-operation error when no current snapshot exists.
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
/// # Parameters
/// - `error`: Query-provider failure to translate.
///
/// # Returns
/// Application-level failure with a stable category.
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
    }
}

/// Extracts a stable subject string from a query target.
///
/// # Parameters
/// - `target`: Query target whose entity identity is needed.
///
/// # Returns
/// Stable textual entity identity.
fn target_subject(target: scriberig_query::TargetRef) -> String {
    match target {
        scriberig_query::TargetRef::Snapshot(value) => value.entity.to_string(),
        scriberig_query::TargetRef::Revision(value) => value.entity.to_string(),
    }
}
