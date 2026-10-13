//! Handles successful candidate commits in the TUI reducer.

use crate::{action::Action, effect::Effect, inspector::LoadState, state::TuiState};

/// Advances the active selection and requests inspection at the committed snapshot.
///
/// # Parameters
/// - `super`: Parent reducer module providing state and effect context.
/// - `state`: Presentation state receiving the authoritative snapshot.
/// - `action`: Successful commit result returned by the application operation.
///
/// # Returns
/// An inspection effect when an active semantic selection exists.
pub(super) fn commit_succeeded(state: &mut TuiState, action: Action) -> Vec<Effect> {
    let Action::CommitSucceeded {
        request_id,
        model,
        snapshot,
    } = action
    else {
        return vec![Effect::None];
    };
    if !matches!(state.review, LoadState::Loading { request_id: current } if current == request_id)
    {
        return vec![Effect::None];
    }
    state.review = LoadState::Ready(model);
    if let Some(project) = state.project.as_mut() {
        let label = project
            .split_once(" @ ")
            .map_or(project.as_str(), |(label, _)| label)
            .to_owned();
        *project = format!("{label} @ {snapshot}");
    }
    let Some(selection) = state.selection.as_mut() else {
        return vec![Effect::None];
    };
    selection.snapshot = Some(snapshot);
    let entity = selection.entity;
    state.explorer = crate::explorer::ExplorerModel::from_selection(*selection);
    let inspect_request = state.next_request_id;
    state.next_request_id += 1;
    state.inspector = LoadState::Loading {
        request_id: inspect_request,
    };
    state.cursor = 0;
    vec![Effect::InspectEntity {
        request_id: inspect_request,
        entity,
        snapshot: Some(snapshot),
    }]
}
