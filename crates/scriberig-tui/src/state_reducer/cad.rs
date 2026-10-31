//! Reduces exact-snapshot CAD navigation and shared layer candidates.

use crate::{action::Action, effect::Effect, inspector::LoadState, state::TuiState};

/// Handles CAD read requests, page results, and layer-change candidates.
///
/// # Parameters
/// - `state`: Presentation state receiving CAD data and candidate lifecycle.
/// - `action`: Action to classify and reduce.
///
/// # Returns
/// CAD effects or shared candidate review effects when the action is recognized.
pub(super) fn reduce(state: &mut TuiState, action: &Action) -> Option<Vec<Effect>> {
    match action {
        Action::CadRequested { .. }
        | Action::CadLoaded { .. }
        | Action::CadFailed { .. }
        | Action::CadEntitySelected { .. } => Some(reduce_read(state, action)),
        Action::CadLayerChangeRequested(_)
        | Action::CadLayerChangeProposed { .. }
        | Action::CadLayerChangeFailed { .. } => Some(reduce_layer_change(state, action)),
        _ => None,
    }
}

/// Reduces exact drawing and bounded entity page lifecycle actions.
///
/// # Parameters
/// - `state`: TUI state receiving the drawing and entity page.
/// - `action`: CAD read request, result, or failure.
///
/// # Returns
/// A bounded inspection effect or a no-op for stale results.
fn reduce_read(state: &mut TuiState, action: &Action) -> Vec<Effect> {
    match action {
        Action::CadRequested {
            project,
            snapshot,
            drawing,
            offset,
        } => {
            let request_id = next_request(state);
            let project = project.to_string();
            let snapshot = snapshot.to_string();
            let drawing = drawing.to_string();
            state.primary_view = crate::selection::PrimaryView::Overview;
            state.cad_drawing = LoadState::Loading { request_id };
            state.cad_entities = LoadState::Loading { request_id };
            state.cad_layer_edit = LoadState::NotRequested;
            state.cad_offset = *offset;
            Some(Effect::InspectCadPage {
                request_id,
                drawing: scriberig_protocol::CadDrawingInspectRequest {
                    project: project.clone(),
                    snapshot: snapshot.clone(),
                    drawing: drawing.clone(),
                },
                entities: scriberig_protocol::CadEntitiesRequest {
                    project,
                    snapshot,
                    drawing,
                    offset: *offset,
                    limit: 100,
                },
            })
            .into_iter()
            .collect()
        }
        Action::CadLoaded {
            request_id,
            drawing,
            entities,
        } => {
            if !matches!(state.cad_drawing, LoadState::Loading { request_id: current } if current == *request_id)
            {
                return vec![Effect::None];
            }
            if drawing.project != entities.project
                || drawing.snapshot != entities.snapshot
                || drawing.drawing != entities.drawing
            {
                set_cad_failure(
                    state,
                    "identity_mismatch",
                    "CAD drawing and entity page do not share one exact snapshot.",
                );
                return vec![Effect::None];
            }
            state.connection = crate::state::ConnectionState::Connected;
            state.stale = false;
            state.cad_drawing = LoadState::Ready(drawing.clone());
            state.cad_entities = LoadState::Ready(entities.clone());
            vec![Effect::None]
        }
        Action::CadFailed {
            request_id,
            category,
            message,
        } => {
            if matches!(state.cad_drawing, LoadState::Loading { request_id: current } if current == *request_id)
            {
                set_cad_failure(state, category, message);
            }
            vec![Effect::None]
        }
        Action::CadEntitySelected { entity } => select_entity(state, *entity),
        _ => vec![Effect::None],
    }
}

/// Selects an entity that belongs to the currently loaded exact CAD page.
///
/// # Parameters
/// - `state`: TUI state containing the loaded drawing and entity page.
/// - `entity`: Semantic identity selected by the user.
///
/// # Returns
/// A no-op effect after selection, or a no-op with a visible validation error.
fn select_entity(state: &mut TuiState, entity: scriberig_core::EntityId) -> Vec<Effect> {
    let (LoadState::Ready(drawing), LoadState::Ready(page)) =
        (&state.cad_drawing, &state.cad_entities)
    else {
        state.conversation_composer.error =
            Some("Load a CAD entity page before selecting an entity.".to_owned());
        return vec![Effect::None];
    };
    if !page
        .entities
        .iter()
        .any(|item| item.id == entity.to_string())
    {
        state.conversation_composer.error =
            Some("That entity is not present in the loaded CAD page.".to_owned());
        return vec![Effect::None];
    }
    let Ok(snapshot) = drawing.snapshot.parse() else {
        state.conversation_composer.error =
            Some("The loaded CAD drawing has an invalid snapshot identity.".to_owned());
        return vec![Effect::None];
    };
    let (Ok(project), Ok(root)) = (
        drawing.project.parse::<scriberig_core::ProjectId>(),
        drawing.drawing.parse::<scriberig_core::EntityId>(),
    ) else {
        state.conversation_composer.error =
            Some("The loaded CAD drawing has invalid project or root identity.".to_owned());
        return vec![Effect::None];
    };
    state.selection = Some(crate::selection::EntitySelection {
        entity,
        revision: None,
        snapshot: Some(snapshot),
    });
    let root = state
        .agent_selection
        .as_ref()
        .filter(|scope| scope.project == Some(project) && scope.snapshot == snapshot)
        .map_or(root, |scope| scope.root);
    state.agent_selection = Some(scriberig_app::AgentSelection {
        project: Some(project),
        root,
        entity,
        snapshot,
    });
    state.primary_view = crate::selection::PrimaryView::Overview;
    vec![Effect::None]
}

/// Reduces layer candidate requests and opens shared review on success.
///
/// # Parameters
/// - `state`: TUI state receiving the layer-change lifecycle.
/// - `action`: Candidate request, result, or failure.
///
/// # Returns
/// Candidate effect or the shared candidate review effects.
fn reduce_layer_change(state: &mut TuiState, action: &Action) -> Vec<Effect> {
    match action {
        Action::CadLayerChangeRequested(request) => {
            if !valid_layer_target(state, request) {
                state.conversation_composer.error = Some(
                    "The CAD entity or unlocked destination layer is not in the loaded exact-snapshot drawing."
                        .to_owned(),
                );
                return vec![Effect::None];
            }
            let request_id = next_request(state);
            state.cad_layer_edit = LoadState::Loading { request_id };
            vec![Effect::ProposeCadLayerChange {
                request_id,
                request: request.clone(),
            }]
        }
        Action::CadLayerChangeProposed {
            request_id,
            candidate,
        } => {
            if matches!(state.cad_layer_edit, LoadState::Loading { request_id: current } if current == *request_id)
            {
                let Ok(base_snapshot) = candidate.base_snapshot.parse() else {
                    set_candidate_failure(state, "CAD candidate has an invalid base snapshot.");
                    return vec![Effect::None];
                };
                state.cad_layer_edit = LoadState::Ready(candidate.clone());
                state.primary_view = crate::selection::PrimaryView::Changes;
                crate::state::state_actions::request_candidate_review(
                    state,
                    scriberig_app::ApplyRequest {
                        base_snapshot,
                        branch: None,
                        change: candidate.candidate.clone(),
                    },
                )
            } else {
                vec![Effect::None]
            }
        }
        Action::CadLayerChangeFailed {
            request_id,
            category,
            message,
        } => {
            if matches!(state.cad_layer_edit, LoadState::Loading { request_id: current } if current == *request_id)
            {
                state.cad_layer_edit = LoadState::Failed {
                    category: category.clone(),
                    message: message.clone(),
                    stale: false,
                };
            }
            vec![Effect::None]
        }
        _ => vec![Effect::None],
    }
}

/// Verifies a layer candidate against the currently loaded exact drawing page.
///
/// # Parameters
/// - `state`: Current CAD drawing, page, and semantic context.
/// - `request`: Exact candidate target and destination layer.
///
/// # Returns
/// `true` only when the drawing is complete, the entity is loaded, and the destination is unlocked.
fn valid_layer_target(
    state: &TuiState,
    request: &scriberig_protocol::CadLayerChangeRequest,
) -> bool {
    let (LoadState::Ready(drawing), LoadState::Ready(entities)) =
        (&state.cad_drawing, &state.cad_entities)
    else {
        return false;
    };
    drawing.complete
        && drawing.project == request.project
        && drawing.snapshot == request.snapshot
        && drawing.drawing == entities.drawing
        && entities.project == request.project
        && entities.snapshot == request.snapshot
        && entities
            .entities
            .iter()
            .any(|entity| entity.id == request.entity)
        && drawing
            .layers
            .iter()
            .any(|layer| layer.id == request.destination_layer && !layer.locked)
}

/// Advances the monotonic TUI request generation.
///
/// # Parameters
/// - `state`: State holding the next request identity.
///
/// # Returns
/// The allocated request identity.
fn next_request(state: &mut TuiState) -> u64 {
    let request = state.next_request_id;
    state.next_request_id = state.next_request_id.saturating_add(1);
    request
}

/// Stores one sanitized failure on both CAD panel states.
///
/// # Parameters
/// - `state`: TUI state receiving the failure.
/// - `category`: Safe failure category.
/// - `message`: Safe presentation message.
fn set_cad_failure(state: &mut TuiState, category: &str, message: &str) {
    if category == "transport_unavailable" {
        state.connection = crate::state::ConnectionState::Disconnected;
        state.stale = true;
    }
    state.cad_drawing = LoadState::Failed {
        category: category.to_owned(),
        message: message.to_owned(),
        stale: false,
    };
    state.cad_entities = LoadState::Failed {
        category: category.to_owned(),
        message: message.to_owned(),
        stale: false,
    };
}

/// Stores an invalid candidate response as a local review failure.
///
/// # Parameters
/// - `state`: TUI state receiving the invalid response.
/// - `message`: Safe explanation for the candidate panel.
fn set_candidate_failure(state: &mut TuiState, message: &str) {
    state.cad_layer_edit = LoadState::Failed {
        category: "invalid_candidate".to_owned(),
        message: message.to_owned(),
        stale: false,
    };
}

#[cfg(test)]
#[path = "cad_tests.rs"]
mod tests;
