//! Reduces exact-snapshot geospatial catalog and feature page actions.

use crate::{action::Action, effect::Effect, inspector::LoadState, state::TuiState};

/// Handles geospatial read requests and exact response generations.
///
/// # Parameters
/// - `state`: Presentation state receiving current catalog and feature pages.
/// - `action`: Action to classify and reduce.
///
/// # Returns
/// Geo service effects for recognized actions, or `None` otherwise.
pub(super) fn reduce(state: &mut TuiState, action: &Action) -> Option<Vec<Effect>> {
    match action {
        Action::GeoCatalogRequested { .. }
        | Action::GeoCatalogLoaded { .. }
        | Action::GeoCatalogFailed { .. } => reduce_catalog(state, action),
        Action::GeoFeaturesRequested { .. }
        | Action::GeoFeaturesLoaded { .. }
        | Action::GeoFeaturesFailed { .. }
        | Action::GeoFeatureSelected { .. } => reduce_features(state, action),
        Action::GeoAttributeEditRequested(_)
        | Action::GeoAttributeEditProposed { .. }
        | Action::GeoAttributeEditFailed { .. } => reduce_edit(state, action),
        Action::GeoGeometryEditRequested(_)
        | Action::GeoGeometryEditProposed { .. }
        | Action::GeoGeometryEditFailed { .. } => reduce_geometry_edit(state, action),
        _ => None,
    }
}

/// Reduces geometry candidate creation and shared-review transitions.
///
/// # Parameters
/// - `state`: TUI state receiving the geometry candidate lifecycle.
/// - `action`: Exact-base geometry request, result, or failure.
///
/// # Returns
/// Geometry candidate effect or common review effects for recognized actions.
fn reduce_geometry_edit(state: &mut TuiState, action: &Action) -> Option<Vec<Effect>> {
    match action {
        Action::GeoGeometryEditRequested(request) => {
            if state
                .geo_snapshot
                .is_none_or(|snapshot| snapshot.to_string() != request.snapshot)
                || state.geo_path.as_deref() != Some(request.path.as_str())
                || !matches!(&state.geo_features, LoadState::Ready(page)
                    if page.layer == request.layer && page.snapshot == request.snapshot
                        && page.features.iter().any(|feature| feature.id == request.feature))
            {
                state.conversation_composer.error = Some(
                    "The geometry target is not present in the loaded exact-snapshot page."
                        .to_owned(),
                );
                return Some(vec![Effect::None]);
            }
            let request_id = next_request(state);
            state.geo_geometry_edit = LoadState::Loading { request_id };
            Some(vec![Effect::ProposeGeoGeometryEdit {
                request_id,
                request: request.clone(),
            }])
        }
        Action::GeoGeometryEditProposed {
            request_id,
            candidate,
        } => {
            if matches!(state.geo_geometry_edit, LoadState::Loading { request_id: current } if current == *request_id)
            {
                state.geo_geometry_edit = LoadState::Ready(candidate.clone());
                state.primary_view = crate::selection::PrimaryView::Changes;
                Some(crate::state::state_actions::request_candidate_review(
                    state,
                    scriberig_app::ApplyRequest {
                        base_snapshot: candidate.base_snapshot,
                        branch: None,
                        change: candidate.id.clone(),
                    },
                ))
            } else {
                Some(vec![Effect::None])
            }
        }
        Action::GeoGeometryEditFailed {
            request_id,
            category,
            message,
        } => {
            if matches!(state.geo_geometry_edit, LoadState::Loading { request_id: current } if current == *request_id)
            {
                state.geo_geometry_edit = LoadState::Failed {
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

/// Reduces candidate creation and common review transitions for feature edits.
///
/// # Parameters
/// - `state`: TUI state receiving the candidate lifecycle.
/// - `action`: Exact-base candidate request, result, or failure.
///
/// # Returns
/// Candidate effect or common review effects for recognized edit actions.
fn reduce_edit(state: &mut TuiState, action: &Action) -> Option<Vec<Effect>> {
    match action {
        Action::GeoAttributeEditRequested(edit) => {
            if state.geo_snapshot != Some(edit.base_snapshot)
                || state.geo_path.as_deref() != Some(edit.path.as_str())
                || !matches!(&state.geo_features, LoadState::Ready(page)
                    if page.layer == edit.layer.to_string()
                        && page.snapshot == edit.base_snapshot.to_string()
                        && page.features.iter().any(|feature| feature.id == edit.feature.to_string()))
            {
                state.conversation_composer.error = Some(
                    "The feature is not present in the loaded exact-snapshot page.".to_owned(),
                );
                return Some(vec![Effect::None]);
            }
            let request_id = next_request(state);
            state.geo_edit = LoadState::Loading { request_id };
            Some(vec![Effect::ProposeGeoAttributeEdit {
                request_id,
                edit: edit.clone(),
            }])
        }
        Action::GeoAttributeEditProposed {
            request_id,
            candidate,
        } => {
            if matches!(state.geo_edit, LoadState::Loading { request_id: current } if current == *request_id)
            {
                state.geo_edit = LoadState::Ready(candidate.clone());
                state.primary_view = crate::selection::PrimaryView::Changes;
                Some(crate::state::state_actions::request_candidate_review(
                    state,
                    scriberig_app::ApplyRequest {
                        base_snapshot: candidate.base_snapshot,
                        branch: None,
                        change: candidate.id.clone(),
                    },
                ))
            } else {
                Some(vec![Effect::None])
            }
        }
        Action::GeoAttributeEditFailed {
            request_id,
            category,
            message,
        } => {
            if matches!(state.geo_edit, LoadState::Loading { request_id: current } if current == *request_id)
            {
                state.geo_edit = LoadState::Failed {
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

/// Reduces catalog lifecycle actions.
///
/// # Parameters
/// - `state`: Presentation state receiving catalog metadata.
/// - `action`: Catalog request, result, or failure.
///
/// # Returns
/// Catalog effect or a no-op effect for recognized catalog actions.
fn reduce_catalog(state: &mut TuiState, action: &Action) -> Option<Vec<Effect>> {
    match action {
        Action::GeoCatalogRequested {
            project,
            path,
            snapshot,
            offset,
        } => {
            let request_id = next_request(state);
            state.primary_view = crate::selection::PrimaryView::Overview;
            state.geo_path = Some(path.clone());
            state.geo_snapshot = Some(*snapshot);
            state.geo_catalog = LoadState::Loading { request_id };
            state.geo_features = LoadState::NotRequested;
            state.geo_query_result = LoadState::NotRequested;
            Some(vec![Effect::InspectGeoCatalog {
                request_id,
                project: *project,
                path: path.clone(),
                snapshot: *snapshot,
                offset: *offset,
            }])
        }
        Action::GeoCatalogLoaded {
            request_id,
            catalog,
        } => {
            if matches!(state.geo_catalog, LoadState::Loading { request_id: current } if current == *request_id)
            {
                state.connection = crate::state::ConnectionState::Connected;
                state.stale = false;
                state.geo_catalog = LoadState::Ready(catalog.clone());
            }
            Some(vec![Effect::None])
        }
        Action::GeoCatalogFailed {
            request_id,
            category,
            message,
        } => {
            fail(
                &mut state.connection,
                &mut state.stale,
                &mut state.geo_catalog,
                *request_id,
                category,
                message,
            );
            Some(vec![Effect::None])
        }
        _ => None,
    }
}

/// Reduces bounded feature page lifecycle actions.
///
/// # Parameters
/// - `state`: Presentation state receiving feature data.
/// - `action`: Feature request, result, or failure.
///
/// # Returns
/// Feature effect or a no-op effect for recognized feature actions.
fn reduce_features(state: &mut TuiState, action: &Action) -> Option<Vec<Effect>> {
    match action {
        Action::GeoFeatureSelected { feature } => Some(select_feature(state, *feature)),
        Action::GeoFeaturesRequested { layer, offset } => {
            let (Some(path), Some(snapshot), Some(project)) = (
                state.geo_path.clone(),
                state.geo_snapshot,
                state
                    .agent_selection
                    .as_ref()
                    .and_then(|selection| selection.project),
            ) else {
                return Some(vec![Effect::None]);
            };
            let LoadState::Ready(catalog) = &state.geo_catalog else {
                return Some(vec![Effect::None]);
            };
            if catalog.project != project.to_string()
                || catalog.snapshot != snapshot.to_string()
                || !catalog.layers.iter().any(|row| row.id == layer.to_string())
            {
                state.conversation_composer.error = Some(
                    "The requested layer is not present in the loaded exact-snapshot catalog."
                        .to_owned(),
                );
                return Some(vec![Effect::None]);
            }
            let request_id = next_request(state);
            state.geo_query_result = LoadState::NotRequested;
            state.geo_features = LoadState::Loading { request_id };
            Some(vec![Effect::InspectGeoFeatures {
                request_id,
                project,
                path,
                snapshot,
                layer: *layer,
                offset: *offset,
            }])
        }
        Action::GeoFeaturesLoaded { request_id, page } => {
            if matches!(state.geo_features, LoadState::Loading { request_id: current } if current == *request_id)
                && state
                    .geo_snapshot
                    .is_some_and(|snapshot| page.snapshot == snapshot.to_string())
            {
                state.connection = crate::state::ConnectionState::Connected;
                state.stale = false;
                state.geo_features = LoadState::Ready(page.clone());
            }
            Some(vec![Effect::None])
        }
        Action::GeoFeaturesFailed {
            request_id,
            category,
            message,
        } => {
            fail(
                &mut state.connection,
                &mut state.stale,
                &mut state.geo_features,
                *request_id,
                category,
                message,
            );
            Some(vec![Effect::None])
        }
        _ => None,
    }
}

/// Selects a feature present in either loaded feature or query results.
///
/// # Parameters
/// - `state`: TUI state receiving the exact semantic selection.
/// - `feature`: Requested semantic feature identity.
///
/// # Returns
/// A no-op effect after selection or a safe error for an unknown feature.
fn select_feature(state: &mut TuiState, feature: scriberig_core::EntityId) -> Vec<Effect> {
    let selected_id = feature.to_string();
    let in_query_page = matches!(
        &state.geo_query_result,
        LoadState::Ready(page) if page.features.iter().any(|row| row.id == selected_id)
    );
    let snapshot_text = match &state.geo_query_result {
        LoadState::Ready(page) if in_query_page => Some(page.snapshot.as_str()),
        _ => match &state.geo_features {
            LoadState::Ready(page) if page.features.iter().any(|row| row.id == selected_id) => {
                Some(page.snapshot.as_str())
            }
            _ => None,
        },
    };
    let Some(snapshot_text) = snapshot_text else {
        state.conversation_composer.error =
            Some("That feature is not present in the loaded exact-snapshot page.".to_owned());
        return vec![Effect::None];
    };
    let Ok(snapshot) = snapshot_text.parse() else {
        state.conversation_composer.error =
            Some("The loaded feature snapshot identity is invalid.".to_owned());
        return vec![Effect::None];
    };
    state.selection = Some(crate::selection::EntitySelection {
        entity: feature,
        revision: None,
        snapshot: Some(snapshot),
    });
    vec![Effect::None]
}

/// Allocates one monotonic request generation.
///
/// # Parameters
/// - `state`: Presentation state whose generation is advanced.
///
/// # Returns
/// The newly assigned request identifier.
fn next_request(state: &mut TuiState) -> u64 {
    let id = state.next_request_id;
    state.next_request_id = state.next_request_id.saturating_add(1);
    id
}

/// Applies a safe failure only when it belongs to the active request.
///
/// # Parameters
/// - `connection`: Current connection state.
/// - `stale`: Whether cached views require refresh.
/// - `load`: Specific load state receiving the matching error.
/// - `request_id`: Request generation associated with the failure.
/// - `category`: Safe structured error category.
/// - `message`: Sanitized message for the terminal view.
fn fail<T>(
    connection: &mut crate::state::ConnectionState,
    stale: &mut bool,
    load: &mut LoadState<T>,
    request_id: u64,
    category: &str,
    message: &str,
) {
    if category == "transport_unavailable" {
        *connection = crate::state::ConnectionState::Disconnected;
        *stale = true;
    }
    if matches!(load, LoadState::Loading { request_id: current } if *current == request_id) {
        *load = LoadState::Failed {
            category: category.to_owned(),
            message: message.to_owned(),
            stale: false,
        };
    }
}

#[cfg(test)]
#[path = "geo_tests.rs"]
mod tests;
