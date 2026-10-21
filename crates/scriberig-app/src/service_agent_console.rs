//! Implements shared model-library reads and session selection operations.

#[path = "service_agent_console_detail.rs"]
mod service_agent_console_detail;
#[path = "service_agent_console_history.rs"]
mod service_agent_console_history;
#[path = "service_agent_console_principal.rs"]
mod service_agent_console_principal;
#[path = "service_agent_console_request.rs"]
mod service_agent_console_request;
#[path = "service_agent_console_selection.rs"]
mod service_agent_console_selection;
use service_agent_console_history::record_selection_change;
use service_agent_console_principal::presentation_principal;
use service_agent_console_selection::{ensure_revision, invalid_selection, selection_error};

use super::service_sessions::{authorize_model_library_read, authorize_session_model_selection};
use crate::session::ToolResultDetail;
use crate::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationService,
    SessionModelSelectionRequest, SessionThinkingSelectionRequest,
};
use scriberig_core::EntityId;
use scriberig_session::{Session, SessionExecutionSelection};

impl<B: ApplicationBackend> crate::AgentConsoleBackend for ApplicationService<B> {
    /// Projects safe model metadata from upstream state for an authorized caller.
    ///
    /// - `context`: Caller capabilities and request context.
    ///
    /// Upstream-derived model descriptor projection.
    ///
    /// Returns permission-denied when the caller cannot use agent models.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn agent_model_library(
        &self,
        context: &ApplicationContext,
    ) -> Result<Vec<rpi_model_runtime::SafeModelSnapshot>, ApplicationError> {
        authorize_model_library_read(context)?;
        let catalog_snapshot = self.model_catalog.snapshot();
        Ok(catalog_snapshot
            .models
            .iter()
            .map(rpi_model_runtime::safe_model_snapshot)
            .collect())
    }

    /// Reads shared user presentation defaults without touching session state.
    ///
    /// - `context`: Caller capabilities and principal identity.
    ///
    /// The current revisioned defaults for this user.
    ///
    /// Returns permission, missing-principal, or store errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn agent_presentation_preferences(
        &self,
        context: &ApplicationContext,
    ) -> Result<crate::AgentPresentationPreferenceSnapshot, ApplicationError> {
        authorize_model_library_read(context)?;
        let principal = presentation_principal(context)?;
        self.presentation_preferences.read(principal)
    }

    /// Updates shared user defaults with optimistic revision protection.
    ///
    /// - `context`: Caller capabilities and principal identity.
    /// - `expected_revision`: Preference revision observed by the caller.
    /// - `preferences`: Complete independent presentation choices.
    ///
    /// The authoritative updated defaults.
    ///
    /// Returns permission, missing-principal, conflict, or store errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn set_agent_presentation_preferences(
        &self,
        context: &ApplicationContext,
        expected_revision: u64,
        preferences: crate::AgentPresentationPreferences,
    ) -> Result<crate::AgentPresentationPreferenceSnapshot, ApplicationError> {
        authorize_model_library_read(context)?;
        let principal = presentation_principal(context)?;
        self.presentation_preferences
            .compare_and_swap(principal, expected_revision, preferences)
    }

    /// Changes the provider/model pair used by future session invocations.
    ///
    /// A canonical current level is retained and clamped by the upstream model.
    ///
    /// - `context`: Caller capabilities and request context.
    /// - `request`: Session, observed revision, provider id, and model id.
    ///
    /// The updated session selection.
    ///
    /// Returns permission, upstream selection validation, stale-revision, or
    /// persistence failures.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn set_session_model(
        &self,
        context: &ApplicationContext,
        request: &SessionModelSelectionRequest,
    ) -> Result<SessionExecutionSelection, ApplicationError> {
        authorize_session_model_selection(context, request.session)?;
        let (mut session, backend_owned) = self.selection_session(context, request.session)?;
        let previous_provider = session.execution.provider_id.clone();
        let previous_model = session.execution.model_id.clone();
        let previous_thinking = session.execution.thinking_level.clone();
        ensure_revision(
            request.expected_revision,
            session.execution.revision,
            request.session,
        )?;
        let (model, effective_thinking) = self.resolve_upstream_model(
            &request.provider_id,
            &request.model_id,
            session.execution.thinking_level.as_deref(),
        )?;
        let thinking = session
            .execution
            .thinking_level
            .as_ref()
            .map(|_| effective_thinking.as_str().to_owned());
        session
            .execution
            .replace(
                request.expected_revision,
                Some(model.provider.clone()),
                Some(model.id.clone()),
                thinking,
            )
            .map_err(|error| selection_error(error, request.session))?;
        if previous_provider != session.execution.provider_id
            || previous_model != session.execution.model_id
        {
            record_selection_change(
                &mut session.execution,
                scriberig_session::AgentActivityKind::ModelSelectionChanged,
                format!("Model selected: {}", model.name),
            );
        }
        if previous_thinking != session.execution.thinking_level {
            let summary = session.execution.thinking_level.as_ref().map_or_else(
                || "Thinking level cleared".to_owned(),
                |level| format!("Thinking level selected: {level}"),
            );
            record_selection_change(
                &mut session.execution,
                scriberig_session::AgentActivityKind::ThinkingLevelChanged,
                summary,
            );
        }
        let selection = session.execution.clone();
        self.persist_selection(context, session, backend_owned, request.expected_revision)?;
        if previous_provider != selection.provider_id || previous_model != selection.model_id {
            self.operations.publish_session_selection_changed(
                request.session,
                scriberig_session::AgentActivityKind::ModelSelectionChanged,
                &format!("Model selected: {}", model.name),
            );
        }
        if previous_thinking != selection.thinking_level {
            self.operations.publish_session_selection_changed(
                request.session,
                scriberig_session::AgentActivityKind::ThinkingLevelChanged,
                &selection.thinking_level.as_ref().map_or_else(
                    || "Thinking level cleared".to_owned(),
                    |level| format!("Thinking level selected: {level}"),
                ),
            );
        }
        self.operations.publish_session_changed(request.session);
        Ok(selection)
    }

    /// Changes the thinking level used by future session invocations.
    ///
    /// - `context`: Caller capabilities and request context.
    /// - `request`: Session, observed selection revision, and thinking id.
    ///
    /// The updated session selection.
    ///
    /// Returns permission, active-model, invalid-level, stale-revision, or
    /// persistence failures.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn set_session_thinking_level(
        &self,
        context: &ApplicationContext,
        request: &SessionThinkingSelectionRequest,
    ) -> Result<SessionExecutionSelection, ApplicationError> {
        authorize_session_model_selection(context, request.session)?;
        let (mut session, backend_owned) = self.selection_session(context, request.session)?;
        let previous_thinking = session.execution.thinking_level.clone();
        ensure_revision(
            request.expected_revision,
            session.execution.revision,
            request.session,
        )?;
        let provider_id = session
            .execution
            .provider_id
            .as_deref()
            .ok_or_else(|| invalid_selection("select a model first"))?;
        let model_id = session
            .execution
            .model_id
            .as_deref()
            .ok_or_else(|| invalid_selection("select a model first"))?;
        let requested = rpi_ai::ThinkingLevel::parse_name(&request.thinking_level)
            .ok_or_else(|| invalid_selection("thinking level id is not supported upstream"))?;
        let (model, effective) =
            self.resolve_upstream_model(provider_id, model_id, Some(requested.as_str()))?;
        let effective = effective.as_str().to_owned();
        session
            .execution
            .replace(
                request.expected_revision,
                Some(model.provider.clone()),
                Some(model.id.clone()),
                Some(effective),
            )
            .map_err(|error| selection_error(error, request.session))?;
        if previous_thinking != session.execution.thinking_level {
            let summary = session.execution.thinking_level.as_ref().map_or_else(
                || "Thinking level cleared".to_owned(),
                |level| format!("Thinking level selected: {level}"),
            );
            record_selection_change(
                &mut session.execution,
                scriberig_session::AgentActivityKind::ThinkingLevelChanged,
                summary,
            );
        }
        let selection = session.execution.clone();
        self.persist_selection(context, session, backend_owned, request.expected_revision)?;
        if previous_thinking != selection.thinking_level {
            self.operations.publish_session_selection_changed(
                request.session,
                scriberig_session::AgentActivityKind::ThinkingLevelChanged,
                &selection.thinking_level.as_ref().map_or_else(
                    || "Thinking level cleared".to_owned(),
                    |level| format!("Thinking level selected: {level}"),
                ),
            );
        }
        self.operations.publish_session_changed(request.session);
        Ok(selection)
    }

    /// Reads bounded safe detail from one durable result.
    /// - `context`: Authorized application request context.
    /// - `session`: Session containing the result.
    /// - `branch`: Branch containing the result.
    /// - `result`: Stable tool-result identity.
    ///
    /// Safe result detail with its stable result identity.
    ///
    /// Returns authorization, lookup, or session persistence errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn tool_result_detail(
        &self,
        context: &ApplicationContext,
        session: EntityId,
        branch: EntityId,
        result: EntityId,
    ) -> Result<ToolResultDetail, ApplicationError> {
        service_agent_console_detail::tool_result_detail(self, context, session, branch, result)
    }

    /// Reads the bounded request projection captured for an invocation.
    ///
    /// - `context`: Authorized request context.
    /// - `session`: Session containing the invocation.
    /// - `branch`: Branch containing the invocation.
    /// - `invocation`: Stable model-invocation identity.
    ///
    /// The captured safe request projection.
    ///
    /// Returns authorization or lookup errors when detail is unavailable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn model_request_detail(
        &self,
        context: &ApplicationContext,
        session: EntityId,
        branch: EntityId,
        invocation: EntityId,
    ) -> Result<scriberig_session::ModelRequestProjection, ApplicationError> {
        service_agent_console_request::model_request_detail(
            self, context, session, branch, invocation,
        )
    }
}

/// Creates an empty session for selection before the first agent message.
///
/// - `id`: Stable identity supplied by the caller.
///
/// A session without branches or semantic work.
/// # Returns
/// The operation result.
fn empty_session(id: EntityId) -> Session {
    Session {
        id,
        branches: Vec::new(),
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: SessionExecutionSelection::default(),
    }
}
