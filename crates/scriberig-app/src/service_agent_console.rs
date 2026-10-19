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
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationService, ModelDescriptor,
    ModelProfile, SessionModelSelectionRequest, SessionThinkingSelectionRequest,
};
use scriberig_core::EntityId;
use scriberig_session::{Session, SessionExecutionSelection};

impl<B: ApplicationBackend> crate::AgentConsoleBackend for ApplicationService<B> {
    /// Lists the configured safe model library for an authorized caller.
    ///
    /// - `context`: Caller capabilities and request context.
    ///
    /// The provider-neutral model descriptors and curated profiles.
    ///
    /// Returns permission-denied when the caller cannot use agent models.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn agent_model_library(
        &self,
        context: &ApplicationContext,
    ) -> Result<crate::ModelLibrary, ApplicationError> {
        authorize_model_library_read(context)?;
        Ok(self.model_library.clone())
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

    /// Changes the model profile used by future session invocations.
    ///
    /// A still-supported thinking level is retained. Otherwise the profile
    /// default, model default, then first advertised level is selected.
    ///
    /// - `context`: Caller capabilities and request context.
    /// - `request`: Session, observed selection revision, and profile id.
    ///
    /// The updated session selection.
    ///
    /// Returns permission, model/profile validation, stale-revision, or
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
        let previous_profile = session.execution.model_profile.clone();
        let previous_thinking = session.execution.thinking_level.clone();
        ensure_revision(
            request.expected_revision,
            session.execution.revision,
            request.session,
        )?;
        let (profile, model) = self.resolve_profile(&request.model_profile)?;
        let thinking =
            resolve_thinking(profile, model, session.execution.thinking_level.as_deref());
        session
            .execution
            .replace(
                request.expected_revision,
                Some(profile.id.clone()),
                Some(model.id.as_str().to_owned()),
                thinking,
            )
            .map_err(|error| selection_error(error, request.session))?;
        if previous_profile != session.execution.model_profile {
            record_selection_change(
                &mut session.execution,
                scriberig_session::AgentActivityKind::ModelSelectionChanged,
                format!("Model selected: {}", model.display_name),
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
        if previous_profile != selection.model_profile {
            self.operations.publish_session_selection_changed(
                request.session,
                scriberig_session::AgentActivityKind::ModelSelectionChanged,
                &format!("Model selected: {}", model.display_name),
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
    /// Returns permission, active-model, advertised-level, stale-revision, or
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
        let profile_id = session
            .execution
            .model_profile
            .as_deref()
            .ok_or_else(|| invalid_selection("select a model profile first"))?;
        let (profile, model) = self.resolve_profile(profile_id)?;
        if !model
            .thinking_levels
            .iter()
            .any(|level| level.id.as_str() == request.thinking_level)
        {
            return Err(invalid_selection(
                "thinking level is not advertised by the selected model",
            ));
        }
        session
            .execution
            .replace(
                request.expected_revision,
                Some(profile.id.clone()),
                Some(model.id.as_str().to_owned()),
                Some(request.thinking_level.clone()),
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

/// Resolves the stable caller identity required for shared preferences.
///
/// - `context`: Authorized application request context.
///
/// The stable principal identifier.
///
/// Returns invalid-request when the caller has no principal identity.
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

/// Chooses a supported thinking level while preserving a valid current choice.
///
/// - `profile`: Curated profile selected for the invocation.
/// - `model`: Descriptor advertising supported levels.
/// - `current`: Current selection, if any.
///
/// The supported current level or deterministic profile/model fallback.
/// # Returns
/// The operation result.
fn resolve_thinking(
    profile: &ModelProfile,
    model: &ModelDescriptor,
    current: Option<&str>,
) -> Option<String> {
    let supports = |candidate: &str| {
        model
            .thinking_levels
            .iter()
            .any(|level| level.id.as_str() == candidate)
    };
    current
        .filter(|value| supports(value))
        .map(str::to_owned)
        .or_else(|| {
            profile
                .default_thinking_level
                .as_ref()
                .map(|level| level.as_str().to_owned())
        })
        .or_else(|| {
            model
                .default_thinking_level
                .as_ref()
                .map(|level| level.as_str().to_owned())
        })
        .or_else(|| {
            model
                .thinking_levels
                .first()
                .map(|level| level.id.as_str().to_owned())
        })
}
