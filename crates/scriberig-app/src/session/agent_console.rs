//! Defines model-console selection and safe activity detail contracts.

use super::{EntityId, SessionExecutionSelection};

/// Identifies one upstream provider/model selection for a session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionModelSelectionRequest {
    /// Stable session identity to update.
    pub session: EntityId,
    /// Selection revision observed by the client.
    pub expected_revision: u64,
    /// Upstream provider identity to select.
    pub provider_id: String,
    /// Provider-scoped upstream model identity to select.
    pub model_id: String,
}

/// Identifies one requested thinking-level mutation for a session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionThinkingSelectionRequest {
    /// Stable session identity to update.
    pub session: EntityId,
    /// Selection revision observed by the client.
    pub expected_revision: u64,
    /// Advertised thinking-level identity to select.
    pub thinking_level: String,
}

/// Holds bounded safe detail for one durable tool result.
///
/// # Public members
/// - result: Stable tool-result identity.
/// - detail: Allowlisted result fields safe for client presentation.
/// - truncated: Whether the detail budget omitted safe values.
///
/// # Private members
/// None.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolResultDetail {
    /// Stable tool-result identity.
    pub result: EntityId,
    /// Whitelisted provider-neutral result fields.
    pub detail: serde_json::Value,
    /// Whether safe fields were truncated to satisfy the response bound.
    pub truncated: bool,
}

/// Owns model-library reads and session-scoped execution selection.
pub trait AgentConsoleBackend {
    /// Lists upstream safe model snapshots for an authorized caller.
    ///
    /// - `context`: Authorized application request context.
    ///
    /// Safe rows projected by the upstream model-runtime crate.
    ///
    /// Returns a permission or configuration failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn agent_model_library(
        &self,
        context: &crate::ApplicationContext,
    ) -> Result<Vec<rpi_model_runtime::SafeModelSnapshot>, crate::ApplicationError>;

    /// Reads the caller's shared presentation defaults.
    ///
    /// - `_context`: Authorized caller context.
    ///
    /// Revisioned shared defaults independent from session history.
    ///
    /// Returns unsupported-operation when unavailable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn agent_presentation_preferences(
        &self,
        _context: &crate::ApplicationContext,
    ) -> Result<crate::AgentPresentationPreferenceSnapshot, crate::ApplicationError> {
        Err(crate::ApplicationError::UnsupportedOperation {
            operation: "agent presentation preferences".to_owned(),
        })
    }

    /// Updates shared presentation defaults using optimistic revision checks.
    ///
    /// - `_context`: Authorized caller context.
    /// - `_expected_revision`: Preference revision observed by the caller.
    /// - `_preferences`: Complete independent presentation choices.
    ///
    /// The updated authoritative preferences.
    ///
    /// Returns unsupported-operation when unavailable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn set_agent_presentation_preferences(
        &self,
        _context: &crate::ApplicationContext,
        _expected_revision: u64,
        _preferences: crate::AgentPresentationPreferences,
    ) -> Result<crate::AgentPresentationPreferenceSnapshot, crate::ApplicationError> {
        Err(crate::ApplicationError::UnsupportedOperation {
            operation: "agent presentation preferences".to_owned(),
        })
    }

    /// Selects a model profile for future session invocations.
    ///
    /// - `context`: Authorized application request context.
    /// - `request`: Session, observed mutation revision, and profile.
    ///
    /// The updated shared session selection.
    ///
    /// Returns a permission, validation, conflict, or persistence failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn set_session_model(
        &self,
        context: &crate::ApplicationContext,
        request: &SessionModelSelectionRequest,
    ) -> Result<SessionExecutionSelection, crate::ApplicationError>;

    /// Selects an advertised thinking level for future invocations.
    ///
    /// - `context`: Authorized application request context.
    /// - `request`: Session, observed revision, and thinking level.
    ///
    /// The updated shared session selection.
    ///
    /// Returns a permission, validation, conflict, or persistence failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn set_session_thinking_level(
        &self,
        context: &crate::ApplicationContext,
        request: &SessionThinkingSelectionRequest,
    ) -> Result<SessionExecutionSelection, crate::ApplicationError>;

    /// Reads allowlisted detail for one durable tool result.
    ///
    /// - `_context`: Authorized application request context.
    /// - `_session`: Session containing the result.
    /// - `_branch`: Branch containing the result.
    /// - `_result`: Stable tool-result identity.
    ///
    /// Bounded safe result detail.
    ///
    /// Returns a permission, lookup, or persistence failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn tool_result_detail(
        &self,
        _context: &crate::ApplicationContext,
        _session: EntityId,
        _branch: EntityId,
        _result: EntityId,
    ) -> Result<ToolResultDetail, crate::ApplicationError> {
        Err(crate::ApplicationError::UnsupportedOperation {
            operation: "tool result detail".to_owned(),
        })
    }

    /// Reads the bounded provider-neutral request captured for an invocation.
    ///
    /// - `_context`: Authorized request context.
    /// - `_session`: Session containing the invocation.
    /// - `_branch`: Branch containing the invocation.
    /// - `_invocation`: Stable model-invocation identity.
    ///
    /// The captured safe request projection.
    ///
    /// Returns unsupported-operation when unavailable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn model_request_detail(
        &self,
        _context: &crate::ApplicationContext,
        _session: EntityId,
        _branch: EntityId,
        _invocation: EntityId,
    ) -> Result<scriberig_session::ModelRequestProjection, crate::ApplicationError> {
        Err(crate::ApplicationError::UnsupportedOperation {
            operation: "model request detail".to_owned(),
        })
    }
}
