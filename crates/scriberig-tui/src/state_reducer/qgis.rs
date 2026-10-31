//! Reduces QGIS project inspection requests and exact-snapshot results.

use super::qgis_order;
use crate::{action::Action, effect::Effect, inspector::LoadState, state::TuiState};

/// Handles QGIS project inspection lifecycle actions.
///
/// # Parameters
/// - `state`: Presentation state receiving the QGIS project result.
/// - `action`: Request, result, or failure to reduce.
///
/// # Returns
/// QGIS inspection effect for recognized actions, or `None` otherwise.
pub(super) fn reduce(state: &mut TuiState, action: &Action) -> Option<Vec<Effect>> {
    match action {
        Action::QgisProjectRequested(request) => {
            let Some(selection) = state.agent_selection.as_ref() else {
                state.conversation_composer.error =
                    Some("QGIS inspection requires an active project.".to_owned());
                return Some(vec![Effect::None]);
            };
            if selection.project != Some(request.project)
                || state.selection.and_then(|selection| selection.snapshot)
                    != Some(request.snapshot)
            {
                state.conversation_composer.error = Some(
                    "QGIS inspection must use the active project and selected exact snapshot."
                        .to_owned(),
                );
                return Some(vec![Effect::None]);
            }
            let request_id = next_request(state);
            state.qgis_project = LoadState::Loading { request_id };
            state.qgis_layer_rename = LoadState::NotRequested;
            state.qgis_layer_order = LoadState::NotRequested;
            state.geo_catalog = LoadState::NotRequested;
            state.geo_features = LoadState::NotRequested;
            state.geo_query_result = LoadState::NotRequested;
            Some(vec![Effect::InspectQgisProject {
                request_id,
                request: request.clone(),
            }])
        }
        Action::QgisProjectLoaded {
            request_id,
            project,
        } => {
            if matches!(state.qgis_project, LoadState::Loading { request_id: current } if current == *request_id)
            {
                state.qgis_project = LoadState::Ready(project.clone());
                state.primary_view = crate::selection::PrimaryView::Overview;
            }
            Some(vec![Effect::None])
        }
        Action::QgisProjectFailed {
            request_id,
            category,
            message,
        } => {
            if matches!(state.qgis_project, LoadState::Loading { request_id: current } if current == *request_id)
            {
                state.qgis_project = LoadState::Failed {
                    category: category.clone(),
                    message: message.clone(),
                    stale: false,
                };
            }
            Some(vec![Effect::None])
        }
        Action::QgisLayerRenameRequested(request) => Some(reduce_rename(state, request)),
        Action::QgisLayerRenameProposed { .. } | Action::QgisLayerRenameFailed { .. } => {
            reduce_rename_result(state, action)
        }
        Action::QgisLayerOrderRequested(request) => Some(qgis_order::reduce_order(state, request)),
        Action::QgisLayerOrderProposed { .. } | Action::QgisLayerOrderFailed { .. } => {
            qgis_order::reduce_order_result(state, action)
        }
        _ => None,
    }
}

/// Reduces exact-snapshot QGIS rename candidate lifecycle actions.
///
/// # Parameters
/// - `state`: Presentation state receiving the candidate lifecycle.
/// - `action`: Candidate request, result, or failure.
/// - `request`: Exact-snapshot candidate request when the action is a request.
///
/// # Returns
/// Candidate effect or shared review effects for recognized actions.
fn reduce_rename(
    state: &mut TuiState,
    request: &scriberig_protocol::QgisLayerRenameRequest,
) -> Vec<Effect> {
    let LoadState::Ready(project) = &state.qgis_project else {
        state.conversation_composer.error =
            Some("Load a QGIS project before renaming a layer.".to_owned());
        return vec![Effect::None];
    };
    let (Some(selection), Some(snapshot)) = (
        state.agent_selection.as_ref(),
        state.selection.and_then(|selection| selection.snapshot),
    ) else {
        state.conversation_composer.error =
            Some("QGIS rename requires an active exact project snapshot.".to_owned());
        return vec![Effect::None];
    };
    if project.project != request.project
        || project.snapshot != request.snapshot
        || selection
            .project
            .is_none_or(|id| id.to_string() != request.project)
        || snapshot.to_string() != request.snapshot
        || !project
            .layers
            .iter()
            .any(|layer| layer.id == request.reference)
        || request.new_name.trim().is_empty()
    {
        state.conversation_composer.error = Some("QGIS rename must target a loaded reference at the active exact snapshot and use a non-empty name.".to_owned());
        return vec![Effect::None];
    }
    let request_id = next_request(state);
    state.qgis_layer_rename = LoadState::Loading { request_id };
    vec![Effect::ProposeQgisLayerRename {
        request_id,
        request: request.clone(),
    }]
}

/// Reduces QGIS rename completion and opens the common candidate review.
///
/// # Parameters
/// - `state`: Presentation state receiving the operation result.
/// - `action`: Candidate result or failure.
///
/// # Returns
/// Shared review effects or a no-op for stale results.
fn reduce_rename_result(state: &mut TuiState, action: &Action) -> Option<Vec<Effect>> {
    match action {
        Action::QgisLayerRenameProposed {
            request_id,
            candidate,
        } => {
            if !matches!(state.qgis_layer_rename, LoadState::Loading { request_id: current } if current == *request_id)
            {
                return Some(vec![Effect::None]);
            }
            let valid_scope = matches!(&state.qgis_project, LoadState::Ready(project)
                if project.project == candidate.project
                    && project.snapshot == candidate.base_snapshot
                    && project.path == candidate.path
                    && project.layers.iter().any(|layer| layer.id == candidate.reference));
            if !valid_scope {
                state.qgis_layer_rename = LoadState::Failed {
                    category: "identity_mismatch".to_owned(),
                    message: "QGIS rename candidate does not match the loaded project reference and snapshot.".to_owned(),
                    stale: false,
                };
                return Some(vec![Effect::None]);
            }
            state.qgis_layer_rename = LoadState::Ready(candidate.clone());
            state.primary_view = crate::selection::PrimaryView::Changes;
            Some(crate::state::state_actions::request_candidate_review(
                state,
                scriberig_app::ApplyRequest {
                    base_snapshot: if let Ok(snapshot) = candidate.base_snapshot.parse() {
                        snapshot
                    } else {
                        state.qgis_layer_rename = LoadState::Failed {
                            category: "invalid_identity".to_owned(),
                            message: "QGIS rename candidate has an invalid base snapshot."
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
        Action::QgisLayerRenameFailed {
            request_id,
            category,
            message,
        } => {
            if matches!(state.qgis_layer_rename, LoadState::Loading { request_id: current } if current == *request_id)
            {
                state.qgis_layer_rename = LoadState::Failed {
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

/// Allocates a request generation that distinguishes stale QGIS responses.
///
/// # Parameters
/// - `state`: TUI state owning the generation counter.
///
/// # Returns
/// The next unique request generation.
pub(super) fn next_request(state: &mut TuiState) -> u64 {
    let request_id = state.next_request_id;
    state.next_request_id = state.next_request_id.saturating_add(1);
    request_id
}

#[cfg(test)]
mod tests {
    use super::reduce;
    use crate::{action::Action, effect::Effect, inspector::LoadState, state::TuiState};
    use scriberig_core::{EntityId, ProjectId, SnapshotId};

    /// Creates QGIS state tied to one active project, snapshot, and reference.
    ///
    /// # Returns
    /// A state initialized with the exact project and loaded QGIS reference.
    fn loaded_state() -> (TuiState, ProjectId, SnapshotId, EntityId) {
        let project = ProjectId::from_bytes([21; 16]);
        let snapshot = SnapshotId::new([22; 32]);
        let reference = EntityId::from_bytes([23; 16]);
        let root = EntityId::from_bytes([24; 16]);
        let state = TuiState {
            agent_selection: Some(scriberig_app::AgentSelection {
                project: Some(project),
                root,
                entity: root,
                snapshot,
            }),
            selection: Some(crate::selection::EntitySelection {
                entity: root,
                revision: None,
                snapshot: Some(snapshot),
            }),
            qgis_project: LoadState::Ready(scriberig_protocol::QgisProjectResponse {
                project: project.to_string(),
                path: "maps/plan.qgs".to_owned(),
                snapshot: snapshot.to_string(),
                revision: "revision-1".to_owned(),
                id: "qgis-project-1".to_owned(),
                title: None,
                crs: None,
                layers: vec![
                    scriberig_protocol::QgisLayerReferenceView {
                        id: reference.to_string(),
                        name: "Parcels".to_owned(),
                        data_source: "parcels.gpkg".to_owned(),
                        path: Some("parcels.gpkg".to_owned()),
                        resolved_path: Some("parcels.gpkg".to_owned()),
                        source_layer: Some("parcels".to_owned()),
                        dataset: None,
                        layer: None,
                        resolution_diagnostic: None,
                        visible: true,
                        renderer: None,
                        scale_range: None,
                    },
                    scriberig_protocol::QgisLayerReferenceView {
                        id: EntityId::from_bytes([25; 16]).to_string(),
                        name: "Roads".to_owned(),
                        data_source: "roads.gpkg".to_owned(),
                        path: Some("roads.gpkg".to_owned()),
                        resolved_path: Some("roads.gpkg".to_owned()),
                        source_layer: Some("roads".to_owned()),
                        dataset: None,
                        layer: None,
                        resolution_diagnostic: None,
                        visible: true,
                        renderer: None,
                        scale_range: None,
                    },
                ],
                groups: vec![scriberig_protocol::QgisLayerGroupView {
                    id: "group-1".to_owned(),
                    name: "root".to_owned(),
                    layers: vec![
                        reference.to_string(),
                        EntityId::from_bytes([25; 16]).to_string(),
                    ],
                }],
            }),
            ..TuiState::default()
        };
        (state, project, snapshot, reference)
    }

    /// Requires exact loaded scope and routes the candidate into shared review.
    #[test]
    fn qgis_rename_candidate_opens_shared_review_at_exact_base() {
        let (mut state, project, snapshot, reference) = loaded_state();
        let request = scriberig_protocol::QgisLayerRenameRequest {
            project: project.to_string(),
            snapshot: snapshot.to_string(),
            reference: reference.to_string(),
            new_name: "Parcels Updated".to_owned(),
        };
        assert!(matches!(
            reduce(&mut state, &Action::QgisLayerRenameRequested(request)),
            Some(effects) if matches!(effects.as_slice(), [Effect::ProposeQgisLayerRename { request_id: 1, .. }])
        ));
        assert!(matches!(
            reduce(
                &mut state,
                &Action::QgisLayerRenameProposed {
                    request_id: 1,
                    candidate: scriberig_protocol::QgisLayerRenameResponse {
                        candidate: "candidate-1".to_owned(),
                        project: project.to_string(),
                        base_snapshot: snapshot.to_string(),
                        reference: reference.to_string(),
                        path: "maps/plan.qgs".to_owned(),
                    },
                },
            ),
            Some(effects) if matches!(effects.as_slice(), [Effect::Apply { request, .. }]
                if request.base_snapshot == snapshot && request.change == "candidate-1")
        ));
        assert_eq!(state.primary_view, crate::selection::PrimaryView::Changes);
    }

    /// Sends exact flat-tree order changes to the shared candidate operation.
    #[test]
    fn qgis_layer_order_request_enters_candidate_flow() {
        let (mut state, project, snapshot, reference) = loaded_state();
        let second = EntityId::from_bytes([25; 16]);
        let request = scriberig_protocol::QgisLayerOrderRequest {
            project: project.to_string(),
            snapshot: snapshot.to_string(),
            path: "maps/plan.qgs".to_owned(),
            ordered_references: vec![second.to_string(), reference.to_string()],
        };

        assert!(matches!(
            reduce(&mut state, &Action::QgisLayerOrderRequested(request.clone())),
            Some(effects) if matches!(effects.as_slice(), [Effect::ProposeQgisLayerOrder { request_id: 1, request: effect_request }] if effect_request == &request)
        ));
        assert!(matches!(
            state.qgis_layer_order,
            LoadState::Loading { request_id: 1 }
        ));
    }
}
