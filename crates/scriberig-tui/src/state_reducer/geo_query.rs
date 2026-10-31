//! Reduces exact-snapshot geospatial query requests and results.

use crate::{action::Action, effect::Effect, inspector::LoadState, state::TuiState};

/// Handles geospatial query lifecycle actions.
///
/// # Parameters
/// - `state`: Presentation state receiving the query result.
/// - `action`: Typed query request, result, or failure.
///
/// # Returns
/// Query effects for recognized actions, otherwise `None`.
pub(super) fn reduce(state: &mut TuiState, action: &Action) -> Option<Vec<Effect>> {
    match action {
        Action::GeoFeatureQueryRequested(request) => {
            let (Some(path), Some(snapshot), Some(project)) = (
                state.geo_path.as_deref(),
                state.geo_snapshot,
                state
                    .agent_selection
                    .as_ref()
                    .and_then(|selection| selection.project),
            ) else {
                state.conversation_composer.error =
                    Some("Load a GeoPackage catalog before querying a layer.".to_owned());
                return Some(vec![Effect::None]);
            };
            let LoadState::Ready(catalog) = &state.geo_catalog else {
                return Some(vec![Effect::None]);
            };
            if request.project != project
                || request.snapshot != snapshot
                || request.path != path
                || catalog.project != project.to_string()
                || catalog.snapshot != snapshot.to_string()
                || !catalog
                    .layers
                    .iter()
                    .any(|layer| layer.id == request.layer.to_string())
            {
                state.conversation_composer.error = Some(
                    "The query must target a layer in the loaded exact-snapshot catalog."
                        .to_owned(),
                );
                return Some(vec![Effect::None]);
            }
            let request_id = next_request(state);
            state.geo_features = LoadState::NotRequested;
            state.geo_query_result = LoadState::Loading { request_id };
            Some(vec![Effect::QueryGeoFeatures {
                request_id,
                request: request.clone(),
            }])
        }
        Action::GeoFeatureQueryLoaded { request_id, result } => {
            if matches!(state.geo_query_result, LoadState::Loading { request_id: current } if current == *request_id)
                && state
                    .geo_snapshot
                    .is_some_and(|snapshot| result.snapshot == snapshot.to_string())
                && state
                    .agent_selection
                    .as_ref()
                    .and_then(|selection| selection.project)
                    .is_some_and(|project| result.project == project.to_string())
            {
                state.connection = crate::state::ConnectionState::Connected;
                state.stale = false;
                state.geo_features = LoadState::Ready(scriberig_protocol::GeoFeaturesResponse {
                    project: result.project.clone(),
                    snapshot: result.snapshot.clone(),
                    layer: result.layer.clone(),
                    features: result.features.clone(),
                    has_more: result.has_more,
                });
                state.geo_query_result = LoadState::Ready(result.clone());
            }
            Some(vec![Effect::None])
        }
        Action::GeoFeatureQueryFailed {
            request_id,
            category,
            message,
        } => {
            if category == "transport_unavailable" {
                state.connection = crate::state::ConnectionState::Disconnected;
                state.stale = true;
            }
            if matches!(state.geo_query_result, LoadState::Loading { request_id: current } if current == *request_id)
            {
                state.geo_query_result = LoadState::Failed {
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

/// Allocates one monotonic TUI request identifier.
///
/// # Parameters
/// - `state`: TUI state owning the request sequence.
///
/// # Returns
/// The identifier assigned to this request.
fn next_request(state: &mut TuiState) -> u64 {
    let id = state.next_request_id;
    state.next_request_id = state.next_request_id.saturating_add(1);
    id
}

#[cfg(test)]
mod tests {
    use super::reduce;
    use crate::{action::Action, effect::Effect, inspector::LoadState, state::TuiState};
    use scriberig_core::{EntityId, ProjectId, SnapshotId};

    /// Routes a typed query request only for a layer in the loaded snapshot catalog.
    #[test]
    fn query_request_emits_exact_scope_effect_and_marks_loading() {
        let project = ProjectId::from_bytes([1; 16]);
        let snapshot = SnapshotId::new([2; 32]);
        let root = EntityId::from_bytes([3; 16]);
        let layer = EntityId::from_bytes([4; 16]);
        let mut state = TuiState {
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
            geo_path: Some("parcels.gpkg".into()),
            geo_snapshot: Some(snapshot),
            geo_catalog: LoadState::Ready(scriberig_protocol::GeoCatalogResponse {
                project: project.to_string(),
                snapshot: snapshot.to_string(),
                dataset: "dataset".into(),
                source_digest: "0".repeat(64),
                layers: vec![scriberig_protocol::GeoLayerView {
                    id: layer.to_string(),
                    dataset: "dataset".into(),
                    name: "parcels".into(),
                    fields: Vec::new(),
                    crs: None,
                    geometry_type: None,
                    feature_count: 0,
                    extent: None,
                    editable: false,
                    diagnostics: Vec::new(),
                }],
                total: 1,
                has_more: false,
            }),
            ..TuiState::default()
        };
        let request = scriberig_app::GeoFeatureQuery {
            project,
            snapshot,
            path: "parcels.gpkg".into(),
            layer,
            predicate: scriberig_domain_geo::GeoPredicate::AttributeEquals {
                field: "parcel_id".into(),
                value: scriberig_domain_geo::GeoValue::Number("124".into()),
            },
            offset: 0,
            limit: 40,
        };
        let effects = reduce(
            &mut state,
            &Action::GeoFeatureQueryRequested(request.clone()),
        )
        .expect("query action");
        assert_eq!(state.geo_query_result, LoadState::Loading { request_id: 1 });
        assert_eq!(
            effects,
            vec![Effect::QueryGeoFeatures {
                request_id: 1,
                request,
            }]
        );
        let feature_id = EntityId::from_bytes([5; 16]);
        reduce(
            &mut state,
            &Action::GeoFeatureQueryLoaded {
                request_id: 1,
                result: scriberig_protocol::GeoFeatureQueryResponse {
                    project: project.to_string(),
                    snapshot: snapshot.to_string(),
                    layer: layer.to_string(),
                    features: vec![scriberig_protocol::GeoFeatureView {
                        id: feature_id.to_string(),
                        native_id: Some("124".into()),
                        geometry: None,
                        attributes: std::collections::BTreeMap::new(),
                    }],
                    has_more: false,
                    source_revision: "revision".into(),
                    source_digest: "a".repeat(64),
                    provider: "geopackage".into(),
                    layer_crs: None,
                    strategy: "streamed_source_pages".into(),
                    coordinate_transform: None,
                    diagnostics: Vec::new(),
                },
            },
        );
        assert!(matches!(state.geo_query_result, LoadState::Ready(_)));
        assert!(
            matches!(state.geo_features, LoadState::Ready(page) if page.features[0].id == feature_id.to_string())
        );
    }
}
