//! Verifies the query protocol contract.

use super::GeoFixture;
use scriberig_app::{
    Capability, CapabilityAction, CapabilityConstraints, CapabilityContext, CapabilityScope,
};
use scriberig_client::Client;
use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_protocol::GeoValue as PublicGeoValue;
use scriberig_server::InProcessTransport;

/// Preserves typed predicate results and provenance across the remote client boundary.
#[test]
fn geo_query_round_trip_keeps_snapshot_predicate_and_provenance() {
    let project = ProjectId::from_bytes([5; 16]);
    let snapshot = SnapshotId::new([6; 32]);
    let layer = EntityId::from_bytes([7; 16]);
    let grant = Capability {
        scope: CapabilityScope::Custom(format!("project:{project}")),
        actions: std::collections::BTreeSet::from([CapabilityAction::Inspect]),
        constraints: CapabilityConstraints::unrestricted(),
    };
    let transport = InProcessTransport::new(GeoFixture {
        project,
        snapshot,
        layer,
        feature: EntityId::from_bytes([9; 16]),
    })
    .with_capabilities(CapabilityContext::with_grants([grant]));
    let result = Client::new(transport)
        .query_geospatial_features(scriberig_protocol::GeoFeatureQueryRequest {
            project: project.to_string(),
            path: "parcels.gpkg".into(),
            snapshot: snapshot.to_string(),
            layer: layer.to_string(),
            predicate: scriberig_protocol::GeoFeaturePredicate::AttributeEquals {
                field: "parcel_id".into(),
                value: PublicGeoValue::Number("124".into()),
            },
            offset: 0,
            limit: 10,
        })
        .expect("remote spatial query");
    assert_eq!(result.project, project.to_string());
    assert_eq!(result.snapshot, snapshot.to_string());
    assert_eq!(result.layer, layer.to_string());
    assert_eq!(result.features.len(), 1);
    assert_eq!(result.features[0].native_id.as_deref(), Some("124"));
    assert_eq!(
        result.source_revision,
        scriberig_core::RevisionId::new([8; 32]).to_string()
    );
    assert_eq!(result.source_digest, "ab".repeat(32));
    assert_eq!(result.provider, "geopackage");
    assert_eq!(
        result
            .layer_crs
            .as_ref()
            .and_then(|crs| crs.code.as_deref()),
        Some("4326")
    );
    assert!(result.coordinate_transform.is_none());
}
