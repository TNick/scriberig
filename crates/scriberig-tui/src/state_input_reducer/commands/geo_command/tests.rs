//! Verifies geospatial command parsing and exact-snapshot edit requests.

use super::{attribute_edit, geometry_edit, query_request};
use crate::{inspector::LoadState, state::TuiState};
use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_protocol::{
    GeoCatalogResponse, GeoFeatureView, GeoFeaturesResponse, GeoFieldType, GeoFieldView,
    GeoLayerView,
};

/// Creates an exact-snapshot TUI fixture with one typed semantic feature.
///
/// # Returns
/// TUI state containing one editable feature in a loaded exact-snapshot page.
fn state() -> TuiState {
    state_with_target(
        ProjectId::from_bytes([1; 16]),
        SnapshotId::new([2; 32]),
        EntityId::from_bytes([4; 16]),
        EntityId::from_bytes([5; 16]),
        "source-124",
    )
}

/// Creates a state from caller-supplied shared semantic target identities.
///
/// # Parameters
/// - `project`: Project owning the semantic feature.
/// - `snapshot`: Exact snapshot containing the feature.
/// - `layer`: Semantic layer identity.
/// - `feature`: Semantic feature identity.
/// - `native_id`: Provider-native feature identifier.
///
/// # Returns
/// TUI state with matching project, snapshot, catalog, and feature page.
fn state_with_target(
    project: ProjectId,
    snapshot: SnapshotId,
    layer: EntityId,
    feature: EntityId,
    native_id: &str,
) -> TuiState {
    let root = EntityId::from_bytes([3; 16]);
    TuiState {
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
        geo_path: Some("parcels.gpkg".to_owned()),
        geo_snapshot: Some(snapshot),
        geo_catalog: LoadState::Ready(GeoCatalogResponse {
            project: project.to_string(),
            snapshot: snapshot.to_string(),
            dataset: "dataset".to_owned(),
            source_digest: "0".repeat(64),
            layers: vec![GeoLayerView {
                id: layer.to_string(),
                dataset: "dataset".to_owned(),
                name: "parcels".to_owned(),
                fields: vec![GeoFieldView {
                    name: "parcel_id".to_owned(),
                    field_type: GeoFieldType::Number,
                    nullable: false,
                }],
                crs: None,
                geometry_type: None,
                feature_count: 1,
                extent: None,
                editable: true,
                diagnostics: Vec::new(),
            }],
            total: 1,
            has_more: false,
        }),
        geo_features: LoadState::Ready(GeoFeaturesResponse {
            project: project.to_string(),
            snapshot: snapshot.to_string(),
            layer: layer.to_string(),
            features: vec![GeoFeatureView {
                id: feature.to_string(),
                native_id: Some(native_id.to_owned()),
                geometry: None,
                attributes: std::collections::BTreeMap::new(),
            }],
            has_more: false,
        }),
        ..TuiState::default()
    }
}

/// Builds a typed request only for a schema-valid exact-snapshot feature edit.
#[test]
fn attribute_edit_validates_feature_field_type_and_identity() {
    let target: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../tests/fixtures/rich-artifact/geo-feature-target.json"
    ))
    .expect("shared web/TUI Geo target fixture");
    let project: ProjectId = target["project"]
        .as_str()
        .expect("project id")
        .parse()
        .expect("valid project id");
    let snapshot: SnapshotId = target["snapshot"]
        .as_str()
        .expect("snapshot id")
        .parse()
        .expect("valid snapshot id");
    let layer: EntityId = target["layer"]
        .as_str()
        .expect("layer id")
        .parse()
        .expect("valid layer id");
    let feature: EntityId = target["feature"]
        .as_str()
        .expect("feature id")
        .parse()
        .expect("valid feature id");
    let field = target["field"].as_str().expect("field name");
    let value = target["value"].as_str().expect("replacement value");
    let state = state_with_target(
        project,
        snapshot,
        layer,
        feature,
        target["nativeId"].as_str().expect("native id"),
    );
    let edit = attribute_edit(&state, &format!("{feature} {field} number:{value}"))
        .expect("valid exact-base number edit");
    assert_eq!(edit.project, project);
    assert_eq!(edit.base_snapshot, snapshot);
    assert_eq!(edit.layer, layer);
    assert_eq!(edit.feature, feature);
    assert_eq!(edit.field, field);
    assert_eq!(
        edit.value,
        scriberig_domain_geo::GeoValue::Number(value.to_owned())
    );
    assert!(attribute_edit(&state, &format!("{feature} parcel_id text:wrong")).is_err());
    assert!(
        attribute_edit(
            &state,
            &format!("{} parcel_id number:5", EntityId::from_bytes([6; 16]))
        )
        .is_err()
    );
}

/// Parses provider-neutral geometry JSON only for a feature in the loaded exact page.
#[test]
fn geometry_edit_preserves_exact_coordinate_text_and_target_scope() {
    let state = state();
    let feature = EntityId::from_bytes([5; 16]);
    let json =
        r#"{"kind":"point","coordinates":{"x":"12.3400","y":"56.78","z":null,"measure":null}}"#;
    let request = geometry_edit(&state, &format!("{feature} {json}"))
        .expect("typed geometry request for loaded feature");
    assert_eq!(request.project, ProjectId::from_bytes([1; 16]).to_string());
    assert_eq!(request.snapshot, SnapshotId::new([2; 32]).to_string());
    assert_eq!(request.layer, EntityId::from_bytes([4; 16]).to_string());
    assert_eq!(
        request.geometry,
        scriberig_protocol::GeoGeometry::Point(scriberig_protocol::GeoCoordinate {
            x: "12.3400".to_owned(),
            y: "56.78".to_owned(),
            z: None,
            measure: None,
        })
    );
}

/// Builds bounded attribute and explicit-CRS query requests from typed JSON.
#[test]
fn query_request_keeps_exact_scope_predicate_and_offset() {
    let state = state();
    let layer = EntityId::from_bytes([4; 16]);
    let query = query_request(
        &state,
        &format!(
            "{layer} {{\"kind\":\"attribute_equals\",\"field\":\"parcel_id\",\"value\":{{\"kind\":\"number\",\"value\":\"124\"}}}} 40"
        ),
    )
    .expect("valid typed query");
    assert_eq!(query.project, ProjectId::from_bytes([1; 16]));
    assert_eq!(query.snapshot, SnapshotId::new([2; 32]));
    assert_eq!(query.path, "parcels.gpkg");
    assert_eq!(query.layer, layer);
    assert_eq!(query.offset, 40);
    assert_eq!(query.limit, 40);
    assert!(matches!(
        query.predicate,
        scriberig_domain_geo::GeoPredicate::AttributeEquals { ref field, .. }
            if field == "parcel_id"
    ));
    assert!(query_request(&state, &format!("{layer} {{invalid}}")).is_err());
}
