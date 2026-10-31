//! Handles history, projection-style, and connection-status commands.

use super::*;

/// Requests history for the exact selected snapshot.
///
/// - `state`: Current reducer state and snapshot selection.
///
/// # Returns
/// The history effect, or a visible error when no exact snapshot is selected.
pub(super) fn history(state: &mut TuiState) -> Vec<Effect> {
    let Some(snapshot) = state.selection.and_then(|selection| selection.snapshot) else {
        state.conversation_composer.error =
            Some("Select an exact snapshot before opening history.".to_owned());
        return vec![Effect::None];
    };
    super::reduce(state, Action::HistoryRequested { snapshot })
}

/// Reads or updates a source projection style through shared application operations.
///
/// - `state`: Current reducer state and semantic selection.
/// - `command`: Full submitted command.
/// - `name`: Registered command name used to isolate its arguments.
///
/// # Returns
/// The shared style effect, or a visible validation error.
pub(super) fn source_style(state: &mut TuiState, command: &str, name: &str) -> Vec<Effect> {
    let args = command.strip_prefix(name).unwrap_or_default().trim();
    if args.eq_ignore_ascii_case("show") {
        let Some(entity) = state.selection.map(|selection| selection.entity) else {
            state.conversation_composer.error =
                Some("Select a source entity before inspecting its effective style.".to_owned());
            return vec![Effect::None];
        };
        return super::reduce(
            state,
            Action::ProjectionStyleResolveRequested(
                super::super::state_reducer::project_source_request::style_resolution_request(
                    entity,
                    state.project.as_deref(),
                ),
            ),
        );
    }
    match source_style_command::preference_request(args) {
        Ok(request) => super::reduce(state, Action::ProjectionStylePreferenceRequested(request)),
        Err(message) => {
            state.conversation_composer.error = Some(message.to_owned());
            vec![Effect::None]
        }
    }
}

/// Reports connection state and the active session identity.
///
/// - `state`: Current reducer state.
///
/// # Returns
/// A presentation-only status effect.
pub(super) fn status(state: &mut TuiState) -> Vec<Effect> {
    state.conversation_composer.error = Some(format!(
        "Connection: {:?} · session: {}",
        state.connection,
        state
            .session_snapshot
            .as_ref()
            .map_or("none".to_owned(), |session| session.id.to_string())
    ));
    vec![Effect::None]
}
