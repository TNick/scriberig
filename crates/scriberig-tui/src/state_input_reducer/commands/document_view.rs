//! Routes semantic document outline paging commands.

use super::*;

/// Loads or pages the selected exact-snapshot document outline.
///
/// # Parameters
/// - `state`: Current reducer state and document selection.
/// - `command`: Submitted outline command and optional row offset.
///
/// # Returns
/// The resulting effect, or no result when the command is unhandled.
pub(super) fn outline(state: &mut TuiState, command: &str) -> Vec<Effect> {
    let offset = match command.split_whitespace().nth(1) {
        Some(value) => {
            if let Ok(offset) = value.parse::<usize>() {
                offset
            } else {
                state.conversation_composer.error =
                    Some("Document outline offset must be a non-negative integer.".to_owned());
                return vec![Effect::None];
            }
        }
        None => 0,
    };
    if command.split_whitespace().count() > 2 {
        state.conversation_composer.error = Some("Usage: /document [row-offset].".to_owned());
        return vec![Effect::None];
    }
    let Some(selection) = state.selection else {
        state.conversation_composer.error =
            Some("Select a semantic document at an exact snapshot first.".to_owned());
        return vec![Effect::None];
    };
    let Some(snapshot) = selection.snapshot else {
        state.conversation_composer.error =
            Some("Document inspection requires an exact snapshot selection.".to_owned());
        return vec![Effect::None];
    };
    let Some(project) = state
        .agent_selection
        .as_ref()
        .and_then(|selection| selection.project)
    else {
        state.conversation_composer.error =
            Some("Document inspection requires an active project.".to_owned());
        return vec![Effect::None];
    };
    let document = if state.document_snapshot == Some(snapshot) {
        state.document_root.unwrap_or(selection.entity)
    } else {
        selection.entity
    };
    if matches!(state.document_tree, crate::inspector::LoadState::Ready(_))
        && state.document_root == Some(document)
        && state.document_snapshot == Some(snapshot)
    {
        return super::reduce(state, Action::DocumentPageSelected { offset });
    }
    if offset > 0 {
        state.conversation_composer.error =
            Some("Load the document first, then request a row offset.".to_owned());
        return vec![Effect::None];
    }
    super::reduce(
        state,
        Action::DocumentRequested(scriberig_app::DocumentTreeQuery {
            project,
            snapshot,
            document,
        }),
    )
}
