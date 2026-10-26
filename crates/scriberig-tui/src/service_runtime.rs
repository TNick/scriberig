//! Defines runtime capability, lifecycle, and event reads for TUI adapters.

use crate::service::ApplicationContext;
use scriberig_app::{ApplicationError, RuntimeCapability};

/// Supplies runtime metadata and tracked operation lifecycle to TUI services.
pub trait TuiRuntimeService {
    /// Returns runtime capabilities exposed by the application composition.
    ///
    /// - `_context`: Application context for capability resolution.
    ///
    /// # Returns
    /// The capabilities available to the TUI.
    ///
    /// # Errors
    /// Returns an application error when capability access fails.
    fn runtime_capabilities(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<RuntimeCapability>, ApplicationError> {
        Ok(Vec::new())
    }

    /// Reads one authoritative operation lifecycle record.
    ///
    /// - `_context`: Application context for the lookup.
    /// - `_operation`: Operation identity to resolve.
    ///
    /// # Returns
    /// The current operation record.
    ///
    /// # Errors
    /// Returns an unsupported-operation or lifecycle access error.
    fn operation_status(
        &self,
        _context: &ApplicationContext,
        _operation: &scriberig_core::OperationId,
    ) -> Result<scriberig_app::OperationRecord, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "operation status".to_owned(),
        })
    }

    /// Requests application-owned cancellation of one operation.
    ///
    /// - `_context`: Application context for the cancellation.
    /// - `_operation`: Operation identity to cancel.
    ///
    /// # Returns
    /// The authoritative cancelled operation record.
    ///
    /// # Errors
    /// Returns an unsupported-operation or lifecycle cancellation error.
    fn cancel_operation(
        &self,
        _context: &ApplicationContext,
        _operation: &scriberig_core::OperationId,
    ) -> Result<scriberig_app::OperationRecord, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "operation cancellation".to_owned(),
        })
    }

    /// Returns a snapshot of the application's tracked operation records.
    ///
    /// - `_context`: Application context for operation visibility.
    ///
    /// # Returns
    /// Current operation records available to this service.
    ///
    /// # Errors
    /// Returns an operation access error when listing is unavailable.
    fn operation_records(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<scriberig_app::OperationRecord>, ApplicationError> {
        Ok(Vec::new())
    }

    /// Returns application events observed since the previous runtime poll.
    ///
    /// - `_context`: Application context for event access.
    ///
    /// # Returns
    /// Retained application events.
    ///
    /// # Errors
    /// Returns an unsupported-operation or event access error.
    fn application_events(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<scriberig_app::ApplicationEvent>, ApplicationError> {
        Ok(Vec::new())
    }
}
