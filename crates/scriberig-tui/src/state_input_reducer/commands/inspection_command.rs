//! Parses exact-snapshot semantic entity inspection commands.

use super::*;

/// Requests inspection for one selected semantic entity and optional snapshot.
///
/// # Parameters
/// - `state`: Current reducer state and semantic selection.
/// - `command`: Full inspection command.
///
/// # Returns
/// The inspection effect, or a visible safe error.
pub(super) fn inspect(state: &mut TuiState, command: &str) -> Vec<Effect> {
    let mut arguments = command.split_whitespace().skip(1);
    let entity_argument = arguments.next();
    let entity = match entity_argument {
        Some(value) => {
            if let Ok(entity) = value.parse() {
                Some(entity)
            } else {
                state.conversation_composer.error =
                    Some("The entity identity is invalid.".to_owned());
                return vec![Effect::None];
            }
        }
        None => state.selection.map(|selection| selection.entity),
    };
    let Some(entity) = entity else {
        state.conversation_composer.error =
            Some("Select an entity or provide its identifier.".to_owned());
        return vec![Effect::None];
    };
    let snapshot = match arguments.next() {
        None => None,
        Some("--snapshot") => {
            if let Some(snapshot) = arguments.next().and_then(|value| value.parse().ok()) {
                Some(snapshot)
            } else {
                state.conversation_composer.error =
                    Some("Use /inspect <entity> --snapshot <snapshot-id>.".to_owned());
                return vec![Effect::None];
            }
        }
        Some(_) => {
            state.conversation_composer.error =
                Some("Use /inspect <entity> [--snapshot <snapshot-id>].".to_owned());
            return vec![Effect::None];
        }
    };
    if arguments.next().is_some() {
        state.conversation_composer.error =
            Some("Use /inspect <entity> [--snapshot <snapshot-id>].".to_owned());
        return vec![Effect::None];
    }
    let request_id = state.next_request_id;
    state.next_request_id = state.next_request_id.saturating_add(1);
    super::reduce(
        state,
        Action::InspectRequested {
            request_id,
            entity,
            snapshot,
            switch_view: true,
        },
    )
}
