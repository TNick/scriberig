//! Verifies provider-neutral geospatial feature protocol contracts.

use std::collections::BTreeMap;

use scriberig_protocol::{
    GeoAttributeEditRequest, GeoAttributeEditResponse, GeoCatalogRequest, GeoCatalogResponse,
    GeoCoordinate, GeoCrsView, GeoFeatureView, GeoFeaturesRequest, GeoFeaturesResponse,
    GeoFieldType, GeoFieldView, GeoGeometry, GeoGeometryEditRequest, GeoGeometryEditResponse,
    GeoGeometryType, GeoLayerView, GeoValue, Request, Response,
};

/// Round-trips exact snapshot and feature values through JSON protocol envelopes.
#[test]
fn geo_feature_request_and_response_preserve_typed_values() {
    let request = Request::GeoFeatures(GeoFeaturesRequest {
        project: "project-1".into(),
        path: "parcels.gpkg".into(),
        snapshot: "snapshot-1".into(),
        layer: "layer-1".into(),
        offset: 5,
        limit: 25,
    });
    let encoded_request = serde_json::to_vec(&request).expect("encode request");
    let decoded_request: Request =
        serde_json::from_slice(&encoded_request).expect("decode request");
    assert_eq!(decoded_request, request);

    let response = Response::GeoFeatures(GeoFeaturesResponse {
        project: "project-1".into(),
        snapshot: "snapshot-1".into(),
        layer: "layer-1".into(),
        features: vec![GeoFeatureView {
            id: "feature-1".into(),
            native_id: Some("124".into()),
            geometry: Some(GeoGeometry::Point(GeoCoordinate {
                x: "1.2300".into(),
                y: "4.5600".into(),
                z: None,
                measure: None,
            })),
            attributes: BTreeMap::from([("parcel_id".into(), GeoValue::Number("124".into()))]),
        }],
        has_more: false,
    });
    let encoded_response = serde_json::to_vec(&response).expect("encode response");
    let decoded_response: Response =
        serde_json::from_slice(&encoded_response).expect("decode response");
    assert_eq!(decoded_response, response);

    let catalog_request = Request::GeoCatalog(GeoCatalogRequest {
        project: "project-1".into(),
        path: "parcels.gpkg".into(),
        snapshot: "snapshot-1".into(),
        offset: 0,
        limit: 20,
    });
    let encoded_catalog_request =
        serde_json::to_vec(&catalog_request).expect("encode catalog request");
    assert_eq!(
        serde_json::from_slice::<Request>(&encoded_catalog_request)
            .expect("decode catalog request"),
        catalog_request
    );
    let catalog_response = Response::GeoCatalog(GeoCatalogResponse {
        project: "project-1".into(),
        snapshot: "snapshot-1".into(),
        dataset: "dataset-1".into(),
        source_digest: "ab".repeat(32),
        layers: vec![GeoLayerView {
            id: "layer-1".into(),
            dataset: "dataset-1".into(),
            name: "parcels".into(),
            fields: vec![GeoFieldView {
                name: "parcel_id".into(),
                field_type: GeoFieldType::Number,
                nullable: false,
            }],
            crs: Some(GeoCrsView {
                authority: Some("EPSG".into()),
                code: Some("4326".into()),
                definition: Some("EPSG:4326".into()),
            }),
            geometry_type: Some(GeoGeometryType::Point),
            feature_count: 2,
            extent: Some(["1".into(), "2".into(), "3".into(), "4".into()]),
            editable: false,
            diagnostics: vec!["read-only".into()],
        }],
        total: 1,
        has_more: false,
    });
    let encoded_catalog_response =
        serde_json::to_vec(&catalog_response).expect("encode catalog response");
    assert_eq!(
        serde_json::from_slice::<Response>(&encoded_catalog_response)
            .expect("decode catalog response"),
        catalog_response
    );
}

/// Round-trips typed exact-base geometry candidate requests and responses.
#[test]
fn geo_geometry_edit_preserves_candidate_target_and_geometry() {
    let request = Request::GeoGeometryEdit(GeoGeometryEditRequest {
        project: "project-1".into(),
        path: "parcels.gpkg".into(),
        snapshot: "snapshot-2".into(),
        layer: "layer-3".into(),
        feature: "feature-4".into(),
        geometry: GeoGeometry::Point(GeoCoordinate {
            x: "10.25".into(),
            y: "20.5".into(),
            z: None,
            measure: None,
        }),
    });
    let encoded = serde_json::to_vec(&request).expect("encode request");
    assert_eq!(
        serde_json::from_slice::<Request>(&encoded).expect("decode request"),
        request
    );

    let response = Response::GeoGeometryEdit(GeoGeometryEditResponse {
        candidate: "candidate-5".into(),
        project: "project-1".into(),
        base_snapshot: "snapshot-2".into(),
        layer: "layer-3".into(),
        feature: "feature-4".into(),
        path: "parcels.gpkg".into(),
    });
    let encoded = serde_json::to_vec(&response).expect("encode response");
    assert_eq!(
        serde_json::from_slice::<Response>(&encoded).expect("decode response"),
        response
    );
}

/// Round-trips typed exact-base attribute candidate requests and responses.
#[test]
fn geo_attribute_edit_preserves_field_value_and_candidate_target() {
    let request = Request::GeoAttributeEdit(GeoAttributeEditRequest {
        project: "project-1".into(),
        path: "parcels.gpkg".into(),
        snapshot: "snapshot-2".into(),
        layer: "layer-3".into(),
        feature: "feature-4".into(),
        field: "area".into(),
        value: GeoValue::Number("42.500".into()),
    });
    let encoded = serde_json::to_vec(&request).expect("encode request");
    assert_eq!(
        serde_json::from_slice::<Request>(&encoded).expect("decode request"),
        request
    );
    let response = Response::GeoAttributeEdit(GeoAttributeEditResponse {
        candidate: "candidate-5".into(),
        project: "project-1".into(),
        base_snapshot: "snapshot-2".into(),
        layer: "layer-3".into(),
        feature: "feature-4".into(),
        field: "area".into(),
        path: "parcels.gpkg".into(),
    });
    let encoded = serde_json::to_vec(&response).expect("encode response");
    assert_eq!(
        serde_json::from_slice::<Response>(&encoded).expect("decode response"),
        response
    );
}
