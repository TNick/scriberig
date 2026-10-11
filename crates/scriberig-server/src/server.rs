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

// Adapt the shared application façade to the protocol server boundary.
impl<B> ApplicationService for scriberig_app::ApplicationService<B>
where
    B: scriberig_app::ApplicationBackend,
{
    /// Executes inspection through the shared application façade.
    ///
    /// # Parameters
    /// - `self`: Shared application service.
    /// - `context`: Trusted request context.
    /// - `request`: Snapshot-aware inspection request.
    ///
    /// # Returns
    /// Application inspection result.
    ///
    /// # Errors
    /// Returns the shared application failure.
    fn inspect(
        &self,
        context: &ApplicationContext,
        request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError> {
        scriberig_app::QueryService::inspect(self, context, request)
    }

    /// Advertises the generic inspect capability supplied by this façade.
    ///
    /// # Returns
    /// One available inspect capability.
    fn capabilities(&self) -> Vec<RuntimeCapability> {
        vec![RuntimeCapability::new(
            scriberig_app::CapabilityId::new("inspect_entity").expect("static capability"),
            "Inspect one entity",
            true,
        )]
    }

    /// Reports that operation persistence is not part of this synchronous façade.
    ///
    /// # Parameters
    /// - `self`: Shared application service.
    /// - `operation_id`: Requested operation identity.
    ///
    /// # Returns
    /// Never returns a completed operation record.
    ///
    /// # Errors
    /// Returns a structured unsupported-operation failure.
    fn operation_status(&self, operation_id: &str) -> Result<Operation, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: format!("operation status {operation_id}"),
        })
    }
}
