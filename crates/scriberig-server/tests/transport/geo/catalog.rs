//! Verifies the catalog protocol contract.

use super::GeoFixture;
use scriberig_app::{
    Capability, CapabilityAction, CapabilityConstraints, CapabilityContext, CapabilityScope,
};
use scriberig_client::Client;
use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_protocol::GeoFieldType;
use scriberig_server::InProcessTransport;

/// Preserves `GeoPackage` layer schema, CRS, count, and fidelity over the client boundary.
#[test]
fn geo_catalog_round_trip_preserves_schema_crs_and_source_digest() {
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
    let catalog = Client::new(transport)
        .inspect_geopackage(
            project.to_string(),
            "parcels.gpkg",
            snapshot.to_string(),
            0,
            10,
        )
        .expect("GeoPackage catalog");
    assert_eq!(catalog.project, project.to_string());
    assert_eq!(catalog.snapshot, snapshot.to_string());
    assert_eq!(catalog.dataset, EntityId::from_bytes([8; 16]).to_string());
    assert_eq!(catalog.source_digest, "ab".repeat(32));
    assert_eq!(catalog.total, 1);
    assert!(!catalog.has_more);
    assert_eq!(catalog.layers[0].name, "parcels");
    assert_eq!(catalog.layers[0].feature_count, 2);
    assert_eq!(catalog.layers[0].fields[0].field_type, GeoFieldType::Number);
    assert_eq!(
        catalog.layers[0].crs.as_ref().unwrap().code.as_deref(),
        Some("4326")
    );
    assert!(!catalog.layers[0].editable);
}
