//! Verifies large `GeoPackage` catalogs and feature reads stay bounded in the application layer.

use std::collections::BTreeSet;
use std::fs;
use std::sync::Arc;

use rusqlite::{Connection, params};
use scriberig_app::{
    ApplicationContext, ApplicationService, Capability, CapabilityAction, CapabilityConstraints,
    CapabilityContext, CapabilityScope,
};
use scriberig_domain_geo::{GeoPredicate, GeoValue};
use scriberig_project::{
    CandidateValidationProfile, GeoPackageFeatureAttributeEdit, ProjectService,
};
use scriberig_store::SnapshotStore;
use scriberig_store_sqlite::SqliteStore;

/// Creates a deterministic point `GeoPackage` containing many parcel rows.
///
/// # Parameters
/// - `feature_count`: Number of point features to write.
///
/// # Returns
/// Serialized `GeoPackage` bytes with one indexed layer and a known extent.
fn large_geopackage(feature_count: u32) -> Vec<u8> {
    let connection = Connection::open_in_memory().expect("fixture database");
    connection
        .execute_batch(
            "CREATE TABLE gpkg_contents (
                table_name TEXT PRIMARY KEY, data_type TEXT NOT NULL,
                identifier TEXT, description TEXT DEFAULT '',
                last_change DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                min_x DOUBLE, min_y DOUBLE, max_x DOUBLE, max_y DOUBLE, srs_id INTEGER
            );
            CREATE TABLE gpkg_geometry_columns (
                table_name TEXT, column_name TEXT, geometry_type_name TEXT,
                srs_id INTEGER, z TINYINT, m TINYINT
            );
            CREATE TABLE gpkg_spatial_ref_sys (
                srs_name TEXT, srs_id INTEGER PRIMARY KEY, organization TEXT,
                organization_coordsys_id INTEGER, definition TEXT, description TEXT
            );
            INSERT INTO gpkg_spatial_ref_sys VALUES
                ('WGS 84',4326,'EPSG',4326,'EPSG:4326','');
            CREATE TABLE parcels (
                fid INTEGER PRIMARY KEY, geom BLOB, parcel_id INTEGER NOT NULL
            );
            INSERT INTO gpkg_contents
                (table_name,data_type,identifier,min_x,min_y,max_x,max_y,srs_id)
                VALUES ('parcels','features','Parcels',0,0,29999,29999,4326);
            INSERT INTO gpkg_geometry_columns
                VALUES ('parcels','geom','POINT',4326,0,0);",
        )
        .expect("create GeoPackage schema");
    {
        let mut statement = connection
            .prepare("INSERT INTO parcels VALUES (?1, ?2, ?1)")
            .expect("prepare feature insert");
        for id in 1..=feature_count {
            statement
                .execute(params![id, point_blob(f64::from(id), f64::from(id))])
                .expect("insert parcel");
        }
    }
    connection
        .serialize(rusqlite::MAIN_DB)
        .expect("serialize GeoPackage")
        .to_vec()
}

/// Encodes a `GeoPackage` point with an exact WGS 84 header.
///
/// # Parameters
/// - `x`: Point easting.
/// - `y`: Point northing.
///
/// # Returns
/// `GeoPackage` geometry bytes for one little-endian point.
fn point_blob(x: f64, y: f64) -> Vec<u8> {
    let mut bytes = b"GP\0\x01".to_vec();
    bytes.extend_from_slice(&4326_i32.to_le_bytes());
    bytes.push(1);
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&x.to_le_bytes());
    bytes.extend_from_slice(&y.to_le_bytes());
    bytes
}

/// Creates an exact-snapshot project Inspect grant.
///
/// # Parameters
/// - `project`: Project covered by the grant.
/// - `snapshot`: Exact immutable snapshot covered by the grant.
///
/// # Returns
/// Application context authorizing project inspection.
fn inspect_context(
    project: scriberig_core::ProjectId,
    snapshot: scriberig_core::SnapshotId,
) -> ApplicationContext {
    ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Custom(format!("project:{project}")),
            actions: BTreeSet::from([CapabilityAction::Inspect]),
            constraints: CapabilityConstraints::unrestricted(),
        }]),
        ..ApplicationContext::default()
    }
}

/// Creates an exact-snapshot project Propose grant for domain operations.
///
/// # Parameters
/// - `project`: Project covered by the grant.
/// - `snapshot`: Exact immutable snapshot covered by the grant.
///
/// # Returns
/// Application context authorizing semantic proposals in the project.
fn propose_context(
    project: scriberig_core::ProjectId,
    snapshot: scriberig_core::SnapshotId,
) -> ApplicationContext {
    ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Custom(format!("project:{project}")),
            actions: BTreeSet::from([CapabilityAction::Propose]),
            constraints: CapabilityConstraints {
                operation: Some(scriberig_app::OperationKind::DomainOperation),
                ..CapabilityConstraints::unrestricted()
            },
        }]),
        ..ApplicationContext::default()
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "This performance integration test exercises fixture creation, import, summary, and bounded paging over one 30,000-feature dataset."
)]
/// Reports a 30,000-feature layer summary and reads only requested feature pages.
#[test]
fn large_geopackage_summary_and_feature_pages_remain_bounded() {
    const FEATURE_COUNT: u32 = 30_000;
    let root = std::env::temp_dir().join(format!(
        "scriberig-app-large-geopackage-{}",
        scriberig_core::EntityId::new()
    ));
    fs::create_dir_all(&root).expect("create project");
    fs::write(root.join("parcels.gpkg"), large_geopackage(FEATURE_COUNT))
        .expect("write GeoPackage");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open database"));
    let projects = ProjectService::new(Arc::clone(&store));
    let scan = projects.scan(None, &root).expect("scan project");
    let imported = projects.import(scan.project.id).expect("import project");
    let service = ApplicationService::new(()).with_project_service(projects);
    let context = inspect_context(scan.project.id, imported.snapshot);

    let catalog = service
        .inspect_geopackage(&context, scan.project.id, "parcels.gpkg", 0, 10)
        .expect("bounded layer catalog");
    assert_eq!(catalog.snapshot, imported.snapshot);
    assert_eq!(catalog.layers.len(), 1);
    assert_eq!(catalog.layers[0].feature_count, u64::from(FEATURE_COUNT));
    assert_eq!(catalog.layers[0].layer.fields.len(), 1);
    assert!(catalog.layers[0].extent.is_some());
    let layer = catalog.layers[0].layer.id;

    let first = service
        .read_geopackage_features(&context, scan.project.id, "parcels.gpkg", layer, 0, 1000)
        .expect("first bounded feature page");
    assert_eq!(first.page.features.len(), 1000);
    assert!(first.page.has_more);
    let last = service
        .read_geopackage_features(
            &context,
            scan.project.id,
            "parcels.gpkg",
            layer,
            29_000,
            1000,
        )
        .expect("last bounded feature page");
    assert_eq!(last.page.features.len(), 1000);
    assert_eq!(last.page.features[0].native_id.as_deref(), Some("29001"));
    assert!(!last.page.has_more);
    let query = service
        .query_geospatial_features(
            &context,
            &scriberig_app::GeoFeatureQuery {
                project: scan.project.id,
                snapshot: imported.snapshot,
                path: "parcels.gpkg".to_owned(),
                layer,
                predicate: GeoPredicate::AttributeEquals {
                    field: "parcel_id".to_owned(),
                    value: GeoValue::Number("20000".to_owned()),
                },
                offset: 0,
                limit: 1,
            },
        )
        .expect("bounded attribute query");
    assert_eq!(query.page.features.len(), 1);
    assert!(!query.page.has_more);
    let target = query.page.features[0].id;
    let before_features = store
        .project_entities(scan.project.id)
        .expect("project features before edit")
        .into_iter()
        .filter(|entity| entity.kind == "geo.feature")
        .map(|entity| {
            (
                entity.entity,
                store
                    .resolve(imported.snapshot, entity.entity)
                    .expect("resolve feature before edit")
                    .expect("feature revision before edit"),
            )
        })
        .collect::<Vec<_>>();
    let candidate = service
        .propose_geo_feature_attribute(
            &propose_context(scan.project.id, imported.snapshot),
            GeoPackageFeatureAttributeEdit {
                project: scan.project.id,
                base_snapshot: imported.snapshot,
                path: "parcels.gpkg".to_owned(),
                layer,
                feature: target,
                field: "parcel_id".to_owned(),
                value: GeoValue::Number("99999".to_owned()),
            },
        )
        .expect("propose one feature edit");
    let project_service = ProjectService::new(Arc::clone(&store));
    let validated = project_service
        .validate_candidate_profile_cancellable(
            &candidate.id,
            CandidateValidationProfile::GeoPackagePackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("validate one feature edit");
    let applied = project_service
        .apply_candidate(&validated)
        .expect("apply one feature edit");
    let changed_features = before_features
        .into_iter()
        .filter_map(|(entity, before)| {
            let after = store
                .resolve(applied.successor_snapshot, entity)
                .expect("resolve feature after edit")
                .expect("feature revision after edit");
            (before != after).then_some(entity)
        })
        .collect::<Vec<_>>();
    assert_eq!(changed_features, vec![target]);

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project");
}
