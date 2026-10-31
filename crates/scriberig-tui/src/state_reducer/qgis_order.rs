//! Reduces exact-snapshot QGIS layer-order candidate actions.

use crate::{action::Action, effect::Effect, inspector::LoadState, state::TuiState};

/// Reduces an exact-snapshot full-order candidate request.
///
/// # Parameters
/// - `state`: Presentation state containing the loaded QGIS project.
/// - `request`: Complete semantic layer-reference permutation.
///
/// # Returns
/// Candidate effect or a safe no-op with an input diagnostic.
pub(super) fn reduce_order(
    state: &mut TuiState,
    request: &scriberig_protocol::QgisLayerOrderRequest,
) -> Vec<Effect> {
    let LoadState::Ready(project) = &state.qgis_project else {
        state.conversation_composer.error =
            Some("Load a QGIS project before changing layer order.".to_owned());
        return vec![Effect::None];
    };
    let (Some(selection), Some(snapshot)) = (
        state.agent_selection.as_ref(),
        state.selection.and_then(|selection| selection.snapshot),
    ) else {
        state.conversation_composer.error =
            Some("QGIS layer order requires an active exact project snapshot.".to_owned());
        return vec![Effect::None];
    };
    let known = project
        .layers
        .iter()
        .map(|layer| layer.id.as_str())
        .collect::<std::collections::HashSet<_>>();
    let requested = request
        .ordered_references
        .iter()
        .map(String::as_str)
        .collect::<std::collections::HashSet<_>>();
    if project.project != request.project
        || project.snapshot != request.snapshot
        || project.path != request.path
        || selection
            .project
            .is_none_or(|id| id.to_string() != request.project)
        || snapshot.to_string() != request.snapshot
        || project.groups.len() != 1
        || project.groups[0].layers.len() != project.layers.len()
        || known.len() != project.layers.len()
        || requested.len() != request.ordered_references.len()
        || known != requested
        || project
            .layers
            .iter()
            .map(|layer| layer.id.as_str())
            .eq(request.ordered_references.iter().map(String::as_str))
    {
        state.conversation_composer.error = Some("QGIS layer order must be a complete permutation for the loaded flat tree at the active exact snapshot.".to_owned());
        return vec![Effect::None];
    }
    let request_id = super::qgis::next_request(state);
    state.qgis_layer_order = LoadState::Loading { request_id };
    vec![Effect::ProposeQgisLayerOrder {
        request_id,
        request: request.clone(),
    }]
}

/// Reduces QGIS order completion and opens the common candidate review.
///
/// # Parameters
/// - `state`: Presentation state receiving the candidate lifecycle.
/// - `action`: Candidate result or failure.
///
/// # Returns
/// Shared review effects or a no-op for stale results.
pub(super) fn reduce_order_result(state: &mut TuiState, action: &Action) -> Option<Vec<Effect>> {
    match action {
        Action::QgisLayerOrderProposed {
            request_id,
            candidate,
        } => {
            if !matches!(state.qgis_layer_order, LoadState::Loading { request_id: current } if current == *request_id)
            {
                return Some(vec![Effect::None]);
            }
            let valid_scope = matches!(&state.qgis_project, LoadState::Ready(project)
                if project.project == candidate.project
                    && project.snapshot == candidate.base_snapshot
                    && project.path == candidate.path);
            if !valid_scope {
                state.qgis_layer_order = LoadState::Failed {
                    category: "identity_mismatch".to_owned(),
                    message:
                        "QGIS layer-order candidate does not match the loaded project and snapshot."
                            .to_owned(),
                    stale: false,
                };
                return Some(vec![Effect::None]);
            }
            state.qgis_layer_order = LoadState::Ready(candidate.clone());
            state.primary_view = crate::selection::PrimaryView::Changes;
            Some(crate::state::state_actions::request_candidate_review(
                state,
                scriberig_app::ApplyRequest {
                    base_snapshot: if let Ok(snapshot) = candidate.base_snapshot.parse() {
                        snapshot
                    } else {
                        state.qgis_layer_order = LoadState::Failed {
                            category: "invalid_identity".to_owned(),
                            message: "QGIS layer-order candidate has an invalid base snapshot."
                                .to_owned(),
                            stale: false,
                        };
                        return Some(vec![Effect::None]);
                    },
                    branch: None,
                    change: candidate.candidate.clone(),
                },
            ))
        }
        Action::QgisLayerOrderFailed {
            request_id,
            category,
            message,
        } => {
            if matches!(state.qgis_layer_order, LoadState::Loading { request_id: current } if current == *request_id)
            {
                state.qgis_layer_order = LoadState::Failed {
                    category: category.clone(),
                    message: message.clone(),
                    stale: false,
                };
            }
            Some(vec![Effect::None])
        }
        _ => None,
    }
}
