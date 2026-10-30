//! Verifies `GeoPackage` edit authorization and candidate-only application behavior.

use std::collections::BTreeSet;
use std::fs;
use std::sync::Arc;

use rusqlite::Connection;
use scriberig_agent_core::{Tool, ToolError};
use scriberig_app::agent_tools::{GEO_ATTRIBUTE_EDIT_TOOL, GeoAttributeEditTool};
use scriberig_app::{
    ApplicationContext, ApplicationError, ApplicationService, Capability, CapabilityAction,
    CapabilityConstraints, CapabilityContext, CapabilityScope, OperationKind,
};
use scriberig_domain_geo::{Coordinate, GeoValue, Geometry};
use scriberig_project::{GeoPackageFeatureAttributeEdit, ProjectService};
use scriberig_store_sqlite::SqliteStore;

/// Creates a project-scoped Propose context for one exact snapshot.
///
/// # Parameters
/// - `project`: Authorized project identity.
/// - `snapshot`: Exact immutable snapshot constraint.
///
/// # Returns
/// Request context with only the scoped Propose capability.
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
                operation: Some(OperationKind::DomainOperation),
                ..CapabilityConstraints::unrestricted()
            },
        }]),
        ..ApplicationContext::default()
    }
}

/// Creates a project-scoped Query context for one exact snapshot.
///
/// # Parameters
/// - `project`: Authorized project identity.
/// - `snapshot`: Exact immutable snapshot constraint.
///
/// # Returns
/// Request context with a query-scoped Inspect grant.
fn query_context(
    project: scriberig_core::ProjectId,
    snapshot: scriberig_core::SnapshotId,
) -> ApplicationContext {
    ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Custom(format!("project:{project}")),
            actions: BTreeSet::from([CapabilityAction::Inspect]),
            constraints: CapabilityConstraints {
                operation: Some(OperationKind::Query),
                ..CapabilityConstraints::unrestricted()
            },
        }]),
        ..ApplicationContext::default()
    }
}

/// Creates a `GeoPackage` fixture with one point feature.
///
/// # Returns
/// Serialized package bytes with one editable point layer.
fn geopackage_fixture() -> Vec<u8> {
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
                ('WGS 84', 4326, 'EPSG', 4326, 'EPSG:4326', '');
            CREATE TABLE parcels (fid INTEGER PRIMARY KEY, geom BLOB, label TEXT, area REAL NOT NULL);
            INSERT INTO gpkg_contents
                (table_name,data_type,identifier,min_x,min_y,max_x,max_y,srs_id)
                VALUES ('parcels','features','Parcels',1,2,3,4,4326);
            INSERT INTO gpkg_geometry_columns
                VALUES ('parcels','geom','POINT',4326,0,0);",
        )
        .expect("package catalog");
    let mut geometry = b"GP\0\x01".to_vec();
    geometry.extend_from_slice(&4326_i32.to_le_bytes());
    geometry.push(1);
    geometry.extend_from_slice(&1_u32.to_le_bytes());
    geometry.extend_from_slice(&2.0_f64.to_le_bytes());
    geometry.extend_from_slice(&3.0_f64.to_le_bytes());
    connection
        .execute(
            "INSERT INTO parcels VALUES (124, ?1, 'Parcel 124', 10.5)",
            rusqlite::params![geometry],
        )
        .expect("insert point feature");
    connection
        .serialize(rusqlite::MAIN_DB)
        .expect("serialize fixture")
        .to_vec()
}

#[allow(
    clippy::too_many_lines,
    reason = "This application integration test verifies authorization, typed geometry candidate creation, and unchanged source bytes in one lifecycle."
)]
/// Routes one feature edit through authorization and returns a pending candidate.
#[test]
fn geo_geometry_candidate_requires_propose_grant_and_leaves_source_unchanged() {
    let root = std::env::temp_dir().join(format!(
        "scriberig-app-geo-edit-{}",
        scriberig_core::EntityId::new()
    ));
    fs::create_dir_all(&root).expect("create project");
    let path = root.join("parcels.gpkg");
    let original = geopackage_fixture();
    fs::write(&path, &original).expect("persist package");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let projects = ProjectService::new(Arc::clone(&store));
    let project = projects.scan(None, &root).expect("scan").project.id;
    let snapshot = projects.import(project).expect("import").snapshot;
    let layer = store
        .project_entities(project)
        .expect("owned entities")
        .into_iter()
        .find(|entity| entity.kind == "geo.layer")
        .expect("parcel layer");
    let feature = projects
        .geopackage_feature_page(project, "parcels.gpkg", snapshot, layer.entity, 0, 1)
        .expect("source feature")
        .features
        .remove(0);
    let application = ApplicationService::new(scriberig_app::StoreInspectionBackend::new(
        Arc::clone(&store),
    ))
    .with_project_service(projects);
    let untrusted = ApplicationContext::default().with_snapshot(snapshot);
    let replacement = Geometry::Point(Coordinate {
        x: "2.5".to_owned(),
        y: "3.5".to_owned(),
        z: None,
        measure: None,
    });

    assert!(matches!(
        application.propose_geo_feature_geometry(
            &untrusted,
            project,
            "parcels.gpkg",
            layer.entity,
            feature.id,
            &replacement,
        ),
        Err(ApplicationError::PermissionDenied { .. })
    ));
    let candidate = application
        .propose_geo_feature_geometry(
            &propose_context(project, snapshot),
            project,
            "parcels.gpkg",
            layer.entity,
            feature.id,
            &replacement,
        )
        .expect("authorized geometry candidate");

    assert_eq!(candidate.feature, feature.id);
    assert_eq!(candidate.layer, layer.entity);
    assert_eq!(candidate.base_snapshot, snapshot);
    assert_eq!(candidate.path, "parcels.gpkg");
    let edit = GeoPackageFeatureAttributeEdit {
        project,
        base_snapshot: snapshot,
        path: "parcels.gpkg".into(),
        layer: layer.entity,
        feature: feature.id,
        field: "area".into(),
        value: GeoValue::Number("11.5".into()),
    };
    assert!(matches!(
        application.propose_geo_feature_attribute(&untrusted, edit.clone()),
        Err(ApplicationError::PermissionDenied { .. })
    ));
    let attribute_candidate = application
        .propose_geo_feature_attribute(&propose_context(project, snapshot), edit.clone())
        .expect("authorized attribute candidate");
    assert_eq!(attribute_candidate.feature, feature.id);
    assert_eq!(attribute_candidate.layer, layer.entity);
    assert_eq!(attribute_candidate.field, "area");
    assert_eq!(attribute_candidate.base_snapshot, snapshot);
    assert_eq!(attribute_candidate.path, "parcels.gpkg");
    let layer_context = ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Entity(layer.entity),
            actions: BTreeSet::from([CapabilityAction::Propose]),
            constraints: CapabilityConstraints {
                snapshot: Some(snapshot),
                operation: Some(OperationKind::DomainOperation),
                ..CapabilityConstraints::unrestricted()
            },
        }]),
        ..ApplicationContext::default()
    };
    let layer_candidate = application
        .propose_geo_feature_attribute(&layer_context, edit)
        .expect("entity-scoped layer proposal");
    assert_eq!(layer_candidate.layer, layer.entity);

    let agent_service = ApplicationService::new(scriberig_app::StoreInspectionBackend::new(
        Arc::clone(&store),
    ))
    .with_project_service(ProjectService::new(Arc::clone(&store)));
    let tool = GeoAttributeEditTool::new(agent_service, project);
    let arguments = serde_json::json!({
        "path": "parcels.gpkg",
        "layer": layer.entity.to_string(),
        "feature": feature.id.to_string(),
        "field": "area",
        "value": serde_json::to_value(GeoValue::Number("12.5".to_owned()))
            .expect("encode typed value"),
    });
    assert!(matches!(
        tool.invoke_named(&untrusted, GEO_ATTRIBUTE_EDIT_TOOL, &arguments),
        Err(ToolError::CapabilityDenied { .. })
    ));
    let tool_result = tool
        .invoke_named(
            &propose_context(project, snapshot),
            GEO_ATTRIBUTE_EDIT_TOOL,
            &arguments,
        )
        .expect("authorized geospatial agent candidate");
    assert_eq!(tool_result.entity, feature.id);
    assert_eq!(tool_result.snapshot, Some(snapshot));
    assert_eq!(tool_result.object_type.as_deref(), Some("geo.candidate"));
    assert_eq!(tool_result.handles.len(), 1);
    assert_eq!(fs::read(&path).expect("live source"), original);

    drop(application);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project");
}

/// Routes an exact geospatial predicate through the shared authorized query service.
#[test]
fn geo_query_requires_exact_inspect_scope_and_returns_feature_provenance() {
    let root = std::env::temp_dir().join(format!(
        "scriberig-app-geo-query-{}",
        scriberig_core::EntityId::new()
    ));
    fs::create_dir_all(&root).expect("create project");
    let path = root.join("parcels.gpkg");
    let source = geopackage_fixture();
    fs::write(&path, &source).expect("persist package");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let projects = ProjectService::new(Arc::clone(&store));
    let project = projects.scan(None, &root).expect("scan").project.id;
    let snapshot = projects.import(project).expect("import").snapshot;
    let layer = store
        .project_entities(project)
        .expect("owned entities")
        .into_iter()
        .find(|entity| entity.kind == "geo.layer")
        .expect("parcel layer")
        .entity;
    let application = ApplicationService::new(scriberig_app::StoreInspectionBackend::new(
        Arc::clone(&store),
    ))
    .with_project_service(projects);
    let request = scriberig_app::GeoFeatureQuery {
        project,
        snapshot,
        path: "parcels.gpkg".to_owned(),
        layer,
        predicate: scriberig_domain_geo::GeoPredicate::AttributeEquals {
            field: "area".to_owned(),
            value: GeoValue::Number("10.5".to_owned()),
        },
        offset: 0,
        limit: 1,
    };

    assert!(matches!(
        scriberig_app::QueryService::query_geospatial_features(
            &application,
            &ApplicationContext::default().with_snapshot(snapshot),
            &request,
        ),
        Err(ApplicationError::PermissionDenied { .. })
    ));
    let mismatched = query_context(project, scriberig_core::SnapshotId::new([200; 32]));
    assert!(matches!(
        scriberig_app::QueryService::query_geospatial_features(&application, &mismatched, &request,),
        Err(ApplicationError::InvalidRequest { .. })
    ));

    let result = scriberig_app::QueryService::query_geospatial_features(
        &application,
        &query_context(project, snapshot),
        &request,
    )
    .expect("authorized bounded query");
    assert_eq!(result.project, project);
    assert_eq!(result.page.snapshot, snapshot);
    assert_eq!(result.page.layer, layer);
    assert_eq!(result.page.features.len(), 1);
    assert_eq!(
        result.page.features[0].attributes.get("area"),
        Some(&GeoValue::Number("10.5".to_owned()))
    );
    assert_ne!(result.page.provenance.source_digest, [0; 32]);

    drop(application);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project");
}
