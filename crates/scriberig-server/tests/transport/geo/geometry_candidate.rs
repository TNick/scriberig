//! Verifies the geometry candidate protocol contract.

use super::GeoFixture;
use scriberig_app::{
    Capability, CapabilityAction, CapabilityConstraints, CapabilityContext, CapabilityScope,
};
use scriberig_client::Client;
use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_protocol::GeoGeometry;
use scriberig_server::InProcessTransport;

/// Returns an exact-target geometry candidate through transport and client layers.
#[test]
fn geo_geometry_candidate_round_trip_uses_host_propose_grant() {
    let project = ProjectId::from_bytes([5; 16]);
    let snapshot = SnapshotId::new([6; 32]);
    let layer = EntityId::from_bytes([7; 16]);
    let feature = EntityId::from_bytes([9; 16]);
    let grant = Capability {
        scope: CapabilityScope::Custom(format!("project:{project}")),
        actions: std::collections::BTreeSet::from([CapabilityAction::Propose]),
        constraints: CapabilityConstraints::unrestricted(),
    };
    let transport = InProcessTransport::new(GeoFixture {
        project,
        snapshot,
        layer,
        feature,
    })
    .with_capabilities(CapabilityContext::with_grants([grant]));
    let response = Client::new(transport)
        .propose_geo_geometry_edit(
            project.to_string(),
            "parcels.gpkg",
            snapshot.to_string(),
            layer.to_string(),
            feature.to_string(),
            GeoGeometry::Point(scriberig_protocol::GeoCoordinate {
                x: "10.25".into(),
                y: "20.5".into(),
                z: None,
                measure: None,
            }),
        )
        .expect("remote geometry candidate");
    assert_eq!(response.candidate, "candidate-1");
    assert_eq!(response.base_snapshot, snapshot.to_string());
    assert_eq!(response.layer, layer.to_string());
    assert_eq!(response.feature, feature.to_string());
}
