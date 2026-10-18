//! Maps application preferences onto the provider-neutral protocol DTO.

/// Converts application preferences into the protocol response shape.
///
/// # Parameters
/// - `snapshot`: Shared application preference revision and values.
///
/// # Returns
/// Protocol preferences with the same revision.
pub(crate) fn presentation_response(
    snapshot: &scriberig_app::AgentPresentationPreferenceSnapshot,
) -> scriberig_protocol::AgentPresentationPreferencesResponse {
    use scriberig_app as app;
    use scriberig_protocol as protocol;
    protocol::AgentPresentationPreferencesResponse {
        revision: snapshot.revision,
        preferences: protocol::PublicAgentPresentationPreferences {
            reasoning: match snapshot.preferences.reasoning {
                app::ReasoningVisibility::Hidden => protocol::PublicReasoningVisibility::Hidden,
                app::ReasoningVisibility::Shown => protocol::PublicReasoningVisibility::Shown,
            },
            tool_invocation: match snapshot.preferences.tool_invocation {
                app::ToolInvocationDetail::Simple => protocol::PublicToolInvocationDetail::Simple,
                app::ToolInvocationDetail::Detailed => {
                    protocol::PublicToolInvocationDetail::Detailed
                }
            },
            tool_output: match snapshot.preferences.tool_output {
                app::ToolOutputVisibility::Hidden => protocol::PublicToolOutputVisibility::Hidden,
                app::ToolOutputVisibility::Summary => protocol::PublicToolOutputVisibility::Summary,
                app::ToolOutputVisibility::Full => protocol::PublicToolOutputVisibility::Full,
            },
            prompt: match snapshot.preferences.prompt {
                app::PromptVisibility::Hidden => protocol::PublicPromptVisibility::Hidden,
                app::PromptVisibility::Shown => protocol::PublicPromptVisibility::Shown,
            },
        },
    }
}
