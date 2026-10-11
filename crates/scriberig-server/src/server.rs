//! Defines the application service boundary accepted by the server adapter.

use scriberig_app::{
    ApplicationContext, ApplicationError, InspectRequest, InspectResult, RuntimeCapability,
};
use scriberig_protocol::Operation;

/// Application semantics supplied to protocol dispatch.
pub trait ApplicationService {
    /// Executes one application inspection.
    ///
    /// # Parameters
    /// - `context`: Trusted request context.
    /// - `request`: Snapshot-aware inspection request.
    ///
    /// # Returns
    /// Application inspection result.
    ///
    /// # Errors
    /// Returns a structured application error.
    fn inspect(
        &self,
        context: &ApplicationContext,
        request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError>;

    /// Returns currently available runtime capabilities.
    ///
    /// # Returns
    /// Runtime capability descriptors.
    fn capabilities(&self) -> Vec<RuntimeCapability>;

    /// Returns one operation status in protocol-independent form.
    ///
    /// # Parameters
    /// - `operation_id`: Stable operation identity.
    ///
    /// # Returns
    /// Operation status ready for protocol mapping.
    ///
    /// # Errors
    /// Returns a structured application error when status is unavailable.
    fn operation_status(&self, operation_id: &str) -> Result<Operation, ApplicationError>;
}
