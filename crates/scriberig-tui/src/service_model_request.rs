//! Defines the optional bounded model-request read port used by the TUI.

use super::{ApplicationContext, ApplicationError, EntityId};

/// Reads captured provider-neutral model request details.
pub trait TuiModelRequestService {
    /// Reads shared display defaults for the trusted application principal.
    ///
    /// - `_context`: Authorized application context.
    ///
    /// Revisioned shared preferences.
    ///
    /// Returns unsupported-operation when unavailable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn agent_presentation_preferences(
        &self,
        _context: &ApplicationContext,
    ) -> Result<scriberig_protocol::AgentPresentationPreferencesResponse, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "agent presentation preferences".to_owned(),
        })
    }

    /// Reads the safe request captured for one model invocation.
    ///
    /// - `_context`: Authorized request context.
    /// - `_session`: Session containing the invocation.
    /// - `_branch`: Branch containing the invocation.
    /// - `_invocation`: Stable model-invocation identity.
    ///
    /// Captured provider-neutral request detail.
    ///
    /// Returns unsupported-operation when unavailable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn model_request_detail(
        &self,
        _context: &ApplicationContext,
        _session: EntityId,
        _branch: EntityId,
        _invocation: EntityId,
    ) -> Result<scriberig_protocol::ModelRequestDetailResponse, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "model request detail".to_owned(),
        })
    }
}
