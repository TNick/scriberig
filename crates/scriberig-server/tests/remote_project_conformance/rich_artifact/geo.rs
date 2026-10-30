//! Compares semantic geospatial reads across embedded and protocol surfaces.

use std::fmt::Write;

use scriberig_app::ApplicationContext;
use scriberig_client::Client;
use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_domain_geo::{GeoPredicate, GeoValue};
use scriberig_protocol::{
    GeoFeaturePredicate, GeoFeatureQueryRequest, GeoValue as ProtocolGeoValue,
};
use scriberig_server::InProcessTransport;

use super::super::TestApplication;

type RemoteClient = Client<InProcessTransport<TestApplication>>;
type EmbeddedService = scriberig_app::ApplicationService<
    scriberig_app::StoreInspectionBackend<std::sync::Arc<scriberig_store_sqlite::SqliteStore>>,
>;

/// Confirms `GeoPackage` catalog and feature pages match across app and client APIs.
///
/// - `embedded`: Local application operation boundary.
/// - `remote`: Reusable client over the server protocol.
/// - `context`: Exact-snapshot Inspect authority.
/// - `project`: Project being inspected.
/// - `snapshot`: Exact imported snapshot.
/// - `layer`: Stable semantic `GeoPackage` layer identity.
pub(super) fn assert_geo_reads(
    embedded: &EmbeddedService,
    remote: &RemoteClient,
    context: &ApplicationContext,
    project: ProjectId,
    snapshot: SnapshotId,
    layer: EntityId,
) {
    let local = embedded
        .inspect_geopackage(context, project, "parcels.gpkg", 0, 20)
        .expect("embedded GeoPackage catalog");
    let remote_catalog = remote
        .inspect_geopackage(
            project.to_string(),
            "parcels.gpkg",
            snapshot.to_string(),
            0,
            20,
        )
        .expect("remote GeoPackage catalog");
    assert_eq!(remote_catalog.project, project.to_string());
    assert_eq!(remote_catalog.snapshot, snapshot.to_string());
    assert_eq!(remote_catalog.dataset, local.dataset.id.to_string());
    assert_eq!(
        remote_catalog.source_digest,
        hex_digest(&local.source_digest)
    );
    assert_eq!(remote_catalog.layers[0].id, layer.to_string());
    assert_eq!(remote_catalog.layers[0].name, local.layers[0].layer.name);
    assert_eq!(remote_catalog.layers[0].editable, local.layers[0].editable);
    assert_eq!(
        remote_catalog.layers[0].diagnostics,
        local.layers[0].diagnostics
    );
    assert!(!remote_catalog.layers[0].editable);
    assert!(remote_catalog.layers[0].diagnostics.contains(
        &"layer has package structures outside the safe geometry writer subset".to_owned()
    ));
    assert_eq!(
        remote_catalog.layers[0].feature_count,
        local.layers[0].feature_count
    );

    let local_features = embedded
        .read_geopackage_features(context, project, "parcels.gpkg", layer, 0, 2)
        .expect("embedded GeoPackage features");
    let remote_features = remote
        .geo_features(
            project.to_string(),
            "parcels.gpkg",
            snapshot.to_string(),
            layer.to_string(),
            0,
            2,
        )
        .expect("remote GeoPackage features");
    assert_eq!(remote_features.project, project.to_string());
    assert_eq!(remote_features.snapshot, snapshot.to_string());
    assert_eq!(remote_features.layer, layer.to_string());
    assert_eq!(
        remote_features.features.len(),
        local_features.page.features.len()
    );
    assert_eq!(
        remote_features.features[0].id,
        local_features.page.features[0].id.to_string()
    );
    assert_eq!(
        remote_features.features[0].attributes["parcel_id"],
        ProtocolGeoValue::Number("124".to_owned())
    );
    assert_geo_attribute_query(embedded, remote, context, project, snapshot, layer);
}

/// Confirms attribute queries return the same exact semantic feature handle.
///
/// - `embedded`: Local application operation boundary.
/// - `remote`: Reusable client over the server protocol.
/// - `context`: Exact-snapshot Inspect authority.
/// - `project`: Project being inspected.
/// - `snapshot`: Exact imported snapshot.
/// - `layer`: Stable semantic `GeoLayer` identity.
fn assert_geo_attribute_query(
    embedded: &EmbeddedService,
    remote: &RemoteClient,
    context: &ApplicationContext,
    project: ProjectId,
    snapshot: SnapshotId,
    layer: EntityId,
) {
    let local = embedded
        .query_geospatial_features(
            context,
            &scriberig_app::GeoFeatureQuery {
                project,
                snapshot,
                path: "parcels.gpkg".into(),
                layer,
                predicate: GeoPredicate::AttributeEquals {
                    field: "parcel_id".into(),
                    value: GeoValue::Number("124".into()),
                },
                offset: 0,
                limit: 10,
            },
        )
        .expect("embedded bounded attribute query");
    let remote = remote
        .query_geospatial_features(GeoFeatureQueryRequest {
            project: project.to_string(),
            path: "parcels.gpkg".into(),
            snapshot: snapshot.to_string(),
            layer: layer.to_string(),
            predicate: GeoFeaturePredicate::AttributeEquals {
                field: "parcel_id".into(),
                value: ProtocolGeoValue::Number("124".into()),
            },
            offset: 0,
            limit: 10,
        })
        .expect("remote bounded attribute query");
    assert_eq!(remote.project, project.to_string());
    assert_eq!(remote.snapshot, snapshot.to_string());
    assert_eq!(remote.layer, layer.to_string());
    assert_eq!(remote.features.len(), local.page.features.len());
    assert_eq!(remote.features.len(), 1);
    assert_eq!(remote.features[0].id, local.page.features[0].id.to_string());
    assert_eq!(
        remote.features[0].attributes["parcel_id"],
        ProtocolGeoValue::Number("124".into())
    );
}

/// Converts exact digest bytes to the wire's lowercase hexadecimal form.
///
/// - `digest`: Exact 32-byte project file digest.
///
/// # Returns
/// Sixty-four lowercase hexadecimal characters.
fn hex_digest(digest: &[u8; 32]) -> String {
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("write digest to string");
    }
    encoded
}
