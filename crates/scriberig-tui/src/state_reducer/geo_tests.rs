//! Tests geospatial reducer candidate and selection transitions.

use super::reduce;
use crate::{action::Action, effect::Effect, inspector::LoadState, state::TuiState};
use scriberig_core::{EntityId, ProjectId, SnapshotId};

/// Requires a verified exact-snapshot feature page before requesting a shared edit candidate.
#[test]
fn geo_attribute_candidate_requires_loaded_feature_and_opens_shared_review() {
    let (mut state, project, snapshot, layer, feature) = loaded_feature_state();
    assert_eq!(
        state.selection.map(|selection| selection.entity),
        Some(feature)
    );
    assert_eq!(
        state.selection.and_then(|selection| selection.snapshot),
        Some(snapshot)
    );
    let edit = scriberig_project::GeoPackageFeatureAttributeEdit {
        project,
        base_snapshot: snapshot,
        path: "parcels.gpkg".to_owned(),
        layer,
        feature,
        field: "parcel_id".to_owned(),
        value: scriberig_domain_geo::GeoValue::Number("125".to_owned()),
    };
    let effects = reduce(&mut state, &Action::GeoAttributeEditRequested(edit.clone()))
        .expect("geo edit action handled");
    assert!(matches!(
        effects.as_slice(),
        [Effect::ProposeGeoAttributeEdit { .. }]
    ));
    assert!(matches!(
        state.geo_edit,
        LoadState::Loading { request_id: 1 }
    ));

    let candidate = scriberig_app::GeoAttributeCandidate {
        id: "candidate-1".to_owned(),
        project,
        base_snapshot: snapshot,
        feature,
        layer,
        field: "parcel_id".to_owned(),
        path: "parcels.gpkg".to_owned(),
        validation: scriberig_project::CandidateValidation::Pending,
    };
    reduce(
        &mut state,
        &Action::GeoAttributeEditProposed {
            request_id: 1,
            candidate,
        },
    )
    .expect("geo candidate result handled");
    assert_eq!(state.primary_view, crate::selection::PrimaryView::Changes);
    assert!(matches!(state.geo_edit, LoadState::Ready(_)));
    assert!(state.review_request.is_some());
}

/// Requires a verified exact-snapshot feature before requesting a geometry candidate.
#[test]
fn geo_geometry_candidate_opens_shared_review() {
    let (mut state, project, snapshot, layer, feature) = loaded_feature_state();
    let geometry_request = scriberig_protocol::GeoGeometryEditRequest {
        project: project.to_string(),
        path: "parcels.gpkg".to_owned(),
        snapshot: snapshot.to_string(),
        layer: layer.to_string(),
        feature: feature.to_string(),
        geometry: scriberig_protocol::GeoGeometry::Point(scriberig_protocol::GeoCoordinate {
            x: "1".to_owned(),
            y: "2".to_owned(),
            z: None,
            measure: None,
        }),
    };
    let effects = reduce(
        &mut state,
        &Action::GeoGeometryEditRequested(geometry_request),
    )
    .expect("geometry edit action handled");
    assert!(matches!(
        effects.as_slice(),
        [Effect::ProposeGeoGeometryEdit { .. }]
    ));
    let LoadState::Loading {
        request_id: geometry_request_id,
    } = state.geo_geometry_edit
    else {
        panic!("geometry edit should be loading");
    };
    let candidate = scriberig_app::GeoGeometryCandidate {
        id: "geometry-candidate-1".to_owned(),
        project,
        base_snapshot: snapshot,
        feature,
        layer,
        path: "parcels.gpkg".to_owned(),
        validation: scriberig_project::CandidateValidation::Pending,
    };
    reduce(
        &mut state,
        &Action::GeoGeometryEditProposed {
            request_id: geometry_request_id,
            candidate,
        },
    )
    .expect("geometry candidate result handled");
    assert!(matches!(state.geo_geometry_edit, LoadState::Ready(_)));
    assert_eq!(state.primary_view, crate::selection::PrimaryView::Changes);
}

/// Builds a selected exact-snapshot feature page for candidate reducer tests.
///
/// # Returns
/// TUI state, project, snapshot, layer, and feature identities for one loaded `GeoPackage` row.
fn loaded_feature_state() -> (TuiState, ProjectId, SnapshotId, EntityId, EntityId) {
    let project = ProjectId::from_bytes([1; 16]);
    let snapshot = SnapshotId::new([2; 32]);
    let layer = EntityId::from_bytes([3; 16]);
    let feature = EntityId::from_bytes([4; 16]);
    let mut state = TuiState {
        geo_path: Some("parcels.gpkg".to_owned()),
        geo_snapshot: Some(snapshot),
        geo_features: LoadState::Ready(scriberig_protocol::GeoFeaturesResponse {
            project: project.to_string(),
            snapshot: snapshot.to_string(),
            layer: layer.to_string(),
            features: vec![scriberig_protocol::GeoFeatureView {
                id: feature.to_string(),
                native_id: None,
                geometry: None,
                attributes: std::collections::BTreeMap::new(),
            }],
            has_more: false,
        }),
        ..TuiState::default()
    };
    reduce(&mut state, &Action::GeoFeatureSelected { feature })
        .expect("feature selection action handled");
    (state, project, snapshot, layer, feature)
}
