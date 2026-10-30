//! Verifies the semantic client emits typed `GeoPackage` geometry candidates.

use scriberig_client::Client;
use scriberig_protocol::{
    GeoAttributeEditRequest, GeoAttributeEditResponse, GeoCoordinate, GeoGeometry,
    GeoGeometryEditResponse, GeoValue, ProtocolTransport, ProtocolVersion, Request,
    RequestEnvelope, Response, ResponseEnvelope,
};

/// Confirms the geometry edit target and returns candidate metadata.
struct FixtureTransport;

impl ProtocolTransport for FixtureTransport {
    /// Checks exact source and semantic target fields, then returns a candidate.
    ///
    /// # Parameters
    /// - `request`: Typed protocol request from the client.
    ///
    /// # Returns
    /// Protocol response envelope for the geometry candidate.
    fn send(&self, request: RequestEnvelope) -> ResponseEnvelope {
        assert!(
            matches!(request.request, Request::GeoGeometryEdit(ref request)
            if request.project == "project-1"
                && request.path == "parcels.gpkg"
                && request.snapshot == "snapshot-2"
                && request.layer == "layer-3"
                && request.feature == "feature-4"
                && request.geometry == GeoGeometry::Point(GeoCoordinate {
                    x: "10".into(), y: "20".into(), z: None, measure: None,
                }))
        );
        ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: request.correlation_id,
            response: Response::GeoGeometryEdit(GeoGeometryEditResponse {
                candidate: "candidate-5".into(),
                project: "project-1".into(),
                base_snapshot: "snapshot-2".into(),
                layer: "layer-3".into(),
                feature: "feature-4".into(),
                path: "parcels.gpkg".into(),
            }),
        }
    }
}

/// Confirms the attribute edit target and returns a candidate.
struct AttributeFixtureTransport;

impl ProtocolTransport for AttributeFixtureTransport {
    /// Checks the typed attribute request and returns candidate metadata.
    ///
    /// # Parameters
    /// - `request`: Typed protocol request from the client.
    ///
    /// # Returns
    /// Protocol response envelope for the attribute candidate.
    fn send(&self, request: RequestEnvelope) -> ResponseEnvelope {
        assert!(
            matches!(request.request, Request::GeoAttributeEdit(ref request)
            if request.project == "project-1"
                && request.path == "parcels.gpkg"
                && request.snapshot == "snapshot-2"
                && request.layer == "layer-3"
                && request.feature == "feature-4"
                && request.field == "area"
                && request.value == GeoValue::Number("42.5".into()))
        );
        ResponseEnvelope {
            version: ProtocolVersion::default(),
            correlation_id: request.correlation_id,
            response: Response::GeoAttributeEdit(GeoAttributeEditResponse {
                candidate: "candidate-5".into(),
                project: "project-1".into(),
                base_snapshot: "snapshot-2".into(),
                layer: "layer-3".into(),
                feature: "feature-4".into(),
                field: "area".into(),
                path: "parcels.gpkg".into(),
            }),
        }
    }
}

/// Returns a typed candidate from the remote attribute edit operation.
#[test]
fn client_returns_geo_attribute_candidate() {
    let candidate = Client::new(AttributeFixtureTransport)
        .propose_geo_attribute_edit(GeoAttributeEditRequest {
            project: "project-1".into(),
            path: "parcels.gpkg".into(),
            snapshot: "snapshot-2".into(),
            layer: "layer-3".into(),
            feature: "feature-4".into(),
            field: "area".into(),
            value: GeoValue::Number("42.5".into()),
        })
        .expect("attribute candidate");
    assert_eq!(candidate.candidate, "candidate-5");
    assert_eq!(candidate.field, "area");
}

/// Returns a typed candidate handle from the remote geometry edit operation.
#[test]
fn client_returns_geo_geometry_candidate() {
    let candidate = Client::new(FixtureTransport)
        .propose_geo_geometry_edit(
            "project-1",
            "parcels.gpkg",
            "snapshot-2",
            "layer-3",
            "feature-4",
            GeoGeometry::Point(GeoCoordinate {
                x: "10".into(),
                y: "20".into(),
                z: None,
                measure: None,
            }),
        )
        .expect("geometry candidate");
    assert_eq!(candidate.candidate, "candidate-5");
    assert_eq!(candidate.feature, "feature-4");
}
