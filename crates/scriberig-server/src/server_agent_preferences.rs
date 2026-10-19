//! Adapts shared display preferences between application and protocol layers.

use scriberig_app::{AgentConsoleBackend, ApplicationBackend, ApplicationContext};

/// Reads shared preferences for the authenticated principal.
///
/// - `service`: Application service owning the preference store.
/// - `context`: Trusted capability and principal context.
///
/// Revisioned protocol preferences.
///
/// Returns authorization, principal, or store errors.
pub(super) fn read<B: ApplicationBackend>(
    service: &scriberig_app::ApplicationService<B>,
    context: &ApplicationContext,
) -> Result<scriberig_protocol::AgentPresentationPreferencesResponse, scriberig_app::ApplicationError>
{
    let snapshot = AgentConsoleBackend::agent_presentation_preferences(service, context)?;
    Ok(scriberig_protocol::AgentPresentationPreferencesResponse {
        revision: snapshot.revision,
        preferences: preferences_response(&snapshot.preferences),
    })
}

/// Replaces shared preferences after checking the observed revision.
///
/// - `service`: Application service owning the preference store.
/// - `context`: Trusted capability and principal context.
/// - `request`: Expected revision and complete preference values.
///
/// The updated revisioned protocol preferences.
///
/// Returns authorization, validation, conflict, or store errors.
pub(super) fn set<B: ApplicationBackend>(
    service: &scriberig_app::ApplicationService<B>,
    context: &ApplicationContext,
    request: &scriberig_protocol::AgentPresentationPreferencesSetRequest,
) -> Result<scriberig_protocol::AgentPresentationPreferencesResponse, scriberig_app::ApplicationError>
{
    let snapshot = AgentConsoleBackend::set_agent_presentation_preferences(
        service,
        context,
        request.expected_revision,
        preferences_from_request(&request.preferences),
    )?;
    Ok(scriberig_protocol::AgentPresentationPreferencesResponse {
        revision: snapshot.revision,
        preferences: preferences_response(&snapshot.preferences),
    })
}

/// Converts app-owned display choices to protocol values.
///
/// - `preferences`: Shared app-owned user defaults.
///
/// Provider-neutral protocol values.
/// # Returns
/// The operation result.
fn preferences_response(
    preferences: &scriberig_app::AgentPresentationPreferences,
) -> scriberig_protocol::PublicAgentPresentationPreferences {
    use scriberig_app as app;
    use scriberig_protocol as protocol;
    protocol::PublicAgentPresentationPreferences {
        reasoning: match preferences.reasoning {
            app::ReasoningVisibility::Hidden => protocol::PublicReasoningVisibility::Hidden,
            app::ReasoningVisibility::Shown => protocol::PublicReasoningVisibility::Shown,
        },
        tool_invocation: match preferences.tool_invocation {
            app::ToolInvocationDetail::Simple => protocol::PublicToolInvocationDetail::Simple,
            app::ToolInvocationDetail::Detailed => protocol::PublicToolInvocationDetail::Detailed,
        },
        tool_output: match preferences.tool_output {
            app::ToolOutputVisibility::Hidden => protocol::PublicToolOutputVisibility::Hidden,
            app::ToolOutputVisibility::Summary => protocol::PublicToolOutputVisibility::Summary,
            app::ToolOutputVisibility::Full => protocol::PublicToolOutputVisibility::Full,
        },
        prompt: match preferences.prompt {
            app::PromptVisibility::Hidden => protocol::PublicPromptVisibility::Hidden,
            app::PromptVisibility::Shown => protocol::PublicPromptVisibility::Shown,
        },
    }
}

/// Converts protocol values into the app-owned preference representation.
///
/// - `preferences`: Complete protocol display choices.
///
/// App-owned values stored independently from session history.
/// # Returns
/// The operation result.
fn preferences_from_request(
    preferences: &scriberig_protocol::PublicAgentPresentationPreferences,
) -> scriberig_app::AgentPresentationPreferences {
    use scriberig_app as app;
    use scriberig_protocol as protocol;
    app::AgentPresentationPreferences {
        reasoning: match preferences.reasoning {
            protocol::PublicReasoningVisibility::Hidden => app::ReasoningVisibility::Hidden,
            protocol::PublicReasoningVisibility::Shown => app::ReasoningVisibility::Shown,
        },
        tool_invocation: match preferences.tool_invocation {
            protocol::PublicToolInvocationDetail::Simple => app::ToolInvocationDetail::Simple,
            protocol::PublicToolInvocationDetail::Detailed => app::ToolInvocationDetail::Detailed,
        },
        tool_output: match preferences.tool_output {
            protocol::PublicToolOutputVisibility::Hidden => app::ToolOutputVisibility::Hidden,
            protocol::PublicToolOutputVisibility::Summary => app::ToolOutputVisibility::Summary,
            protocol::PublicToolOutputVisibility::Full => app::ToolOutputVisibility::Full,
        },
        prompt: match preferences.prompt {
            protocol::PublicPromptVisibility::Hidden => app::PromptVisibility::Hidden,
            protocol::PublicPromptVisibility::Shown => app::PromptVisibility::Shown,
        },
    }
}
