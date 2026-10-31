//! Verifies the feature page protocol contract.

use super::GeoFixture;
use scriberig_app::{
    Capability, CapabilityAction, CapabilityConstraints, CapabilityContext, CapabilityScope,
};
use scriberig_client::Client;
use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_protocol::{GeoGeometry, GeoValue as PublicGeoValue};
use scriberig_server::InProcessTransport;

/// Preserves exact feature values and authority through protocol and client.
#[test]
fn geo_feature_page_round_trip_keeps_snapshot_geometry_and_attributes() {
    let project = ProjectId::from_bytes([5; 16]);
    let snapshot = SnapshotId::new([6; 32]);
    let layer = EntityId::from_bytes([7; 16]);
    let feature = EntityId::from_bytes([9; 16]);
    let grant = Capability {
        scope: CapabilityScope::Custom(format!("project:{project}")),
        actions: std::collections::BTreeSet::from([CapabilityAction::Inspect]),
        constraints: CapabilityConstraints::unrestricted(),
    };
    let transport = InProcessTransport::new(GeoFixture {
        project,
        snapshot,
        layer,
        feature,
    })
    .with_capabilities(CapabilityContext::with_grants([grant]));
    let page = Client::new(transport)
        .geo_features(
            project.to_string(),
            "parcels.gpkg",
            snapshot.to_string(),
            layer.to_string(),
            1,
            25,
        )
        .expect("feature page");
    assert_eq!(page.project, project.to_string());
    assert_eq!(page.snapshot, snapshot.to_string());
    assert_eq!(page.layer, layer.to_string());
    assert!(page.has_more);
    assert_eq!(page.features[0].native_id.as_deref(), Some("124"));
    assert_eq!(
        page.features[0].geometry,
        Some(GeoGeometry::Point(scriberig_protocol::GeoCoordinate {
            x: "2.000".into(),
            y: "3.000".into(),
            z: None,
            measure: None,
        }))
    );
    assert_eq!(
        page.features[0].attributes.get("parcel_id"),
        Some(&PublicGeoValue::Number("124".into()))
    );
}
