//! Verifies bounded exact-snapshot attribute and envelope queries over `GeoPackage` sources.

#[path = "support/geopackage_query.rs"]
mod support;

use std::fs;
use std::sync::Arc;

use scriberig_core::EntityId;
use scriberig_domain_geo::{GeoBounds, GeoPredicate, GeoValue};
use scriberig_project::ProjectService;
use scriberig_store::{RevisionStore, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one integration scenario verifies exact-snapshot query and provenance behavior together"
)]
/// Queries bounded features and proves snapshot, CRS, and source provenance are retained.
fn geopackage_query_returns_bounded_snapshot_bound_feature_summaries() {
    let root = std::env::temp_dir().join(format!("scriberig-geopackage-query-{}", EntityId::new()));
    fs::create_dir_all(&root).expect("create project directory");
    let package = support::package();
    fs::write(root.join("parcels.gpkg"), &package).expect("write fixture");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open store"));
    let service = ProjectService::new(Arc::clone(&store));
    let project = service.scan(None, &root).expect("scan").project.id;
    let imported = service.import(project).expect("import");
    let feature_entities = store
        .project_entities(project)
        .expect("imported project entities")
        .into_iter()
        .filter(|entity| entity.kind == "geo.feature")
        .collect::<Vec<_>>();
    assert_eq!(feature_entities.len(), 2);
    let feature_record = feature_entities
        .iter()
        .find(|entity| entity.name.as_deref() == Some("124"))
        .expect("feature 124 semantic revision");
    let revision = store
        .resolve(imported.snapshot, feature_record.entity)
        .expect("resolve feature revision")
        .expect("feature revision exists in imported snapshot");
    let revision = store
        .get(revision)
        .expect("read feature revision")
        .expect("feature revision is present");
    let semantic_feature: scriberig_domain_geo::Feature =
        serde_json::from_slice(revision.payload()).expect("canonical feature payload");
    assert_eq!(semantic_feature.id, feature_record.entity);
    assert_eq!(semantic_feature.native_id.as_deref(), Some("124"));
    assert_eq!(
        semantic_feature.attributes.get("parcel_id"),
        Some(&GeoValue::Number("124".to_owned()))
    );
    let layer = store
        .project_entities(project)
        .expect("project entities")
        .into_iter()
        .find(|entity| entity.kind == "geo.layer")
        .expect("parcel layer")
        .entity;

    let attribute = service
        .query_geopackage_features(&scriberig_project::GeoPackageFeatureQuery {
            project,
            path: "parcels.gpkg".into(),
            snapshot: imported.snapshot,
            layer,
            predicate: GeoPredicate::AttributeEquals {
                field: "parcel_id".to_owned(),
                value: GeoValue::Number("124".to_owned()),
            },
            offset: 0,
            limit: 1,
        })
        .expect("attribute query");
    assert_eq!(attribute.snapshot, imported.snapshot);
    assert_eq!(attribute.layer, layer);
    assert_eq!(attribute.features.len(), 1);
    assert_eq!(
        attribute.features[0].attributes.get("parcel_id"),
        Some(&GeoValue::Number("124".to_owned()))
    );
    assert!(!attribute.has_more);
    assert_eq!(
        attribute.provenance.source_digest,
        *blake3::hash(&package).as_bytes()
    );
    assert_eq!(
        attribute.provenance.strategy,
        scriberig_project::GeoQueryStrategy::StreamedSourcePages
    );
    assert!(attribute.provenance.coordinate_transform.is_none());

    let envelope = GeoPredicate::EnvelopeOverlaps {
        bounds: GeoBounds {
            min_x: "0".to_owned(),
            min_y: "0".to_owned(),
            max_x: "10".to_owned(),
            max_y: "10".to_owned(),
            crs: scriberig_domain_geo::CoordinateReferenceSystem {
                authority: Some("EPSG".to_owned()),
                code: Some("4326".to_owned()),
                definition: Some("EPSG:4326".to_owned()),
            },
        },
    };
    let spatial = service
        .query_geopackage_features(&scriberig_project::GeoPackageFeatureQuery {
            project,
            path: "parcels.gpkg".into(),
            snapshot: imported.snapshot,
            layer,
            predicate: envelope.clone(),
            offset: 0,
            limit: 1,
        })
        .expect("explicit-CRS envelope query");
    assert_eq!(spatial.features.len(), 1);
    assert_eq!(
        spatial.features[0].attributes.get("parcel_id"),
        Some(&GeoValue::Number("124".to_owned()))
    );

    let mut wrong_crs = envelope;
    if let GeoPredicate::EnvelopeOverlaps { bounds } = &mut wrong_crs {
        bounds.crs.code = Some("3857".to_owned());
    }
    assert!(
        service
            .query_geopackage_features(&scriberig_project::GeoPackageFeatureQuery {
                project,
                path: "parcels.gpkg".into(),
                snapshot: imported.snapshot,
                layer,
                predicate: wrong_crs,
                offset: 0,
                limit: 1,
            })
            .is_err()
    );

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove test database");
    fs::remove_dir_all(root).expect("remove test project");
}
