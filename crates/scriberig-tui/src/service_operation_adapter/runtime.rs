//! Implements runtime monitoring for the embedded application adapter.

use super::ApplicationOperationAdapter;
use crate::service::{ApplicationContext, TuiRuntimeService};
use scriberig_app::{ApplicationBackend, ApplicationError, OperationRecord, RuntimeCapability};

impl<B: ApplicationBackend> TuiRuntimeService for ApplicationOperationAdapter<B> {
    /// Reports semantic operations exposed by the embedded application.
    ///
    /// - `_context`: Authorization context used by the surrounding runtime.
    ///
    /// # Returns
    /// Capability descriptors for the command palette.
    ///
    /// # Errors
    /// Returns an application error when capability discovery fails.
    fn runtime_capabilities(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<RuntimeCapability>, ApplicationError> {
        super::service_operation_helpers::runtime_capabilities(self)
    }

    /// Reads an operation record from the shared application registry.
    ///
    /// - `_context`: Application context for the lookup.
    /// - `operation`: Operation identity to resolve.
    ///
    /// # Returns
    /// The current operation record.
    ///
    /// # Errors
    /// Returns a lookup error when the operation is unknown.
    fn operation_status(
        &self,
        _context: &ApplicationContext,
        operation: &scriberig_core::OperationId,
    ) -> Result<OperationRecord, ApplicationError> {
        super::service_operation_helpers::operation_status(self, operation)
    }

    /// Cancels an operation through the shared application registry.
    ///
    /// - `_context`: Application context for the cancellation.
    /// - `operation`: Operation identity to cancel.
    ///
    /// # Returns
    /// The cancelled operation record.
    ///
    /// # Errors
    /// Returns a cancellation error when the operation is unknown.
    fn cancel_operation(
        &self,
        _context: &ApplicationContext,
        operation: &scriberig_core::OperationId,
    ) -> Result<OperationRecord, ApplicationError> {
        super::service_operation_helpers::cancel_operation(self, operation)
    }

    /// Lists current shared application operation records.
    ///
    /// - `_context`: Application context for operation visibility.
    ///
    /// # Returns
    /// Current operation records.
    ///
    /// # Errors
    /// This local adapter has no separate listing failure path.
    fn operation_records(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<OperationRecord>, ApplicationError> {
        Ok(self.service.operations().list())
    }

    /// Returns lifecycle events retained by the shared application registry.
    ///
    /// - `_context`: Application context for event access.
    ///
    /// # Returns
    /// Retained application events.
    ///
    /// # Errors
    /// This local adapter has no separate event access failure path.
    fn application_events(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<scriberig_app::ApplicationEvent>, ApplicationError> {
        Ok(super::service_operation_helpers::application_events(self))
    }
}
