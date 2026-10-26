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

/// Maps one preserved legacy session choice to an exact current upstream selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacyExecutionMappingRequest {
    /// Stable session identity to update.
    pub session: EntityId,
    /// Selection revision observed before the mapping.
    pub expected_revision: u64,
    /// Exact legacy record path returned by recovery inspection.
    pub legacy_path: String,
    /// Current upstream provider identity supplied by the operator.
    pub provider_id: String,
    /// Current provider-scoped upstream model identity supplied by the operator.
    pub model_id: String,
    /// Optional advertised current thinking-level identity supplied by the operator.
    pub thinking_level: Option<String>,
}

/// Maps one current legacy plan-node assignment to a catalog-backed selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacyPlanAssignmentMappingRequest {
    /// Stable session identity containing the plan.
    pub session: EntityId,
    /// Exact legacy recovery record path.
    pub legacy_path: String,
    /// Plan revision observed by the operator.
    pub expected_plan_revision: u64,
    /// Current upstream provider identity supplied by the operator.
    pub provider_id: String,
    /// Provider-scoped current model identity supplied by the operator.
    pub model_id: String,
    /// Optional advertised current thinking-level identity supplied by the operator.
    pub thinking_level: Option<String>,
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

    /// Lists preserved legacy selections requiring an explicit current-model mapping.
    ///
    /// - `context`: Authorized application request context.
    /// - `session`: Stable session identity to inspect.
    ///
    /// # Returns
    /// The unresolved records or an application error.
    ///
    /// # Errors
    /// Returns permission, unsupported-operation, or persistence errors.
    fn legacy_execution_records(
        &self,
        context: &crate::ApplicationContext,
        session: EntityId,
    ) -> Result<Vec<scriberig_session::LegacyExecutionRecord>, crate::ApplicationError> {
        let _ = (context, session);
        Err(crate::ApplicationError::UnsupportedOperation {
            operation: "legacy execution recovery inspection".to_owned(),
        })
    }

    /// Resolves a legacy session choice after validating the operator's exact upstream pair.
    ///
    /// - `context`: Authorized application request context.
    /// - `request`: Legacy path, observed revision, and current upstream selection.
    ///
    /// # Returns
    /// The persisted selection after recovery metadata is removed atomically.
    ///
    /// # Errors
    /// Returns permission, catalog validation, revision, recovery, or persistence errors.
    fn map_legacy_execution_selection(
        &self,
        context: &crate::ApplicationContext,
        request: &LegacyExecutionMappingRequest,
    ) -> Result<SessionExecutionSelection, crate::ApplicationError> {
        let _ = (context, request);
        Err(crate::ApplicationError::UnsupportedOperation {
            operation: "legacy execution selection mapping".to_owned(),
        })
    }

    /// Maps a current legacy plan-node assignment through the active catalog.
    ///
    /// - `context`: Authorized session and plan mutation context.
    /// - `request`: Exact legacy path, plan revision, and explicit current model.
    ///
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns authorization, catalog, revision, node-state, or storage errors.
    fn map_legacy_plan_assignment(
        &self,
        context: &crate::ApplicationContext,
        request: &LegacyPlanAssignmentMappingRequest,
    ) -> Result<scriberig_session::Plan, crate::ApplicationError> {
        let _ = (context, request);
        Err(crate::ApplicationError::UnsupportedOperation {
            operation: "legacy plan assignment mapping".to_owned(),
        })
    }

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
