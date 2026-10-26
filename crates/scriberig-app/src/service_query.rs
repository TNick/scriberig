//! Implements shared semantic queries through the application service.

use super::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, ApplicationService,
    service_query_helpers::{inspect_result, query_request},
};
use crate::{InspectRequest, InspectResult, QueryService};

impl<B: ApplicationBackend> QueryService for ApplicationService<B> {
    /// Executes an application inspection request through the universal path.
    ///
    /// - `self`: Application service.
    /// - `context`: Caller and capability context.
    /// - `request`: Snapshot-aware application inspection request.
    ///
    /// Application inspection metadata in the CLI/server-facing shape.
    ///
    /// Returns a structured application failure when the operation fails or
    /// returns a non-inspection result.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn inspect(
        &self,
        context: &ApplicationContext,
        request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError> {
        let current = match request.snapshot {
            crate::SnapshotResolution::Current => Some(self.backend.current_snapshot(context)?),
            _ => None,
        };
        let application_request = ApplicationRequest::Inspect(query_request(request, current)?);
        let result = self.execute(context, &application_request)?;
        let ApplicationResult::Inspect(inspection) = result else {
            return Err(ApplicationError::Internal {
                message: "inspect backend returned a different result kind".to_owned(),
            });
        };
        inspect_result(inspection, request.detail)
    }

    /// Executes inspection through the application backend's cancellable path.
    ///
    /// - `self`: Application service.
    /// - `context`: Caller and capability context.
    /// - `request`: Snapshot-aware application inspection request.
    /// - `cancelled`: Caller-owned cancellation flag.
    ///
    /// Application inspection metadata when the read completes.
    ///
    /// # Returns
    /// The inspection result.
    ///
    /// # Errors
    /// Returns cancellation, authorization, storage, or result-kind failures.
    fn inspect_cancellable(
        &self,
        context: &ApplicationContext,
        request: &InspectRequest,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<InspectResult, ApplicationError> {
        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
            return Err(ApplicationError::OperationCancelled {
                operation: "inspect".to_owned(),
            });
        }
        let current = match request.snapshot {
            crate::SnapshotResolution::Current => Some(self.backend.current_snapshot(context)?),
            _ => None,
        };
        let application_request = ApplicationRequest::Inspect(query_request(request, current)?);
        let result = self.execute_cancellable(context, &application_request, cancelled)?;
        let ApplicationResult::Inspect(inspection) = result else {
            return Err(ApplicationError::Internal {
                message: "inspect backend returned a different result kind".to_owned(),
            });
        };
        inspect_result(inspection, request.detail)
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
