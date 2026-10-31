//! Verifies semantic document candidates through the remote protocol client.

use std::collections::BTreeSet;
use std::io::{Cursor, Write};
use std::sync::Arc;

use scriberig_app::{
    ApplicationService, Capability, CapabilityAction, CapabilityConstraints, CapabilityContext,
    CapabilityScope, StoreInspectionBackend,
};
use scriberig_client::Client;
use scriberig_core::EntityId;
use scriberig_protocol::{
    CadBounds2D, CadBoundsQueryRequest, CadCandidateValidation, CadCoordinate, CadDrawingSpace,
    DocumentCandidateValidation,
};
use scriberig_server::InProcessTransport;
use scriberig_store::{RevisionStore, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;
use zip::write::SimpleFileOptions;

/// Verifies remote DOCX edits return candidate metadata and preserve the live source.
#[test]
fn document_candidate_round_trip_uses_host_grant_and_shared_project_operation() {
    let root = std::env::temp_dir().join(format!("scriberig-remote-docx-{}", EntityId::new()));
    std::fs::create_dir_all(&root).expect("create project");
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    archive
        .start_file("word/document.xml", SimpleFileOptions::default())
        .expect("document entry");
    archive
        .write_all(br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Existing site area</w:t></w:r></w:p></w:body></w:document>"#)
        .expect("document XML");
    archive
        .start_file("word/styles.xml", SimpleFileOptions::default())
        .expect("styles entry");
    archive
        .write_all(br#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:styleId="RemoteBody"><w:name w:val="Remote Body"/></w:style></w:styles>"#)
        .expect("styles XML");
    let original = archive.finish().expect("finish DOCX").into_inner();
    let path = root.join("report.docx");
    std::fs::write(&path, &original).expect("write DOCX");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let projects = scriberig_project::ProjectService::new(Arc::clone(&store));
    let scan = projects.scan(None, &root).expect("scan project");
    let project = scan.project.id;
    let imported = projects.import(project).expect("import DOCX");
    let paragraph = store
        .project_entities(project)
        .expect("entity ownership")
        .into_iter()
        .filter(|entity| entity.kind == "document.paragraph")
        .find_map(|entity| {
            let revision = store
                .resolve(imported.snapshot, entity.entity)
                .ok()
                .flatten()?;
            let revision = store.get(revision).ok().flatten()?;
            let payload: serde_json::Value = serde_json::from_slice(revision.payload()).ok()?;
            (payload["runs"][0]["text"] == "Existing site area").then_some(entity.entity)
        })
        .expect("semantic paragraph");
    let document = store
        .project_entities(project)
        .expect("entity ownership")
        .into_iter()
        .find(|entity| entity.kind == "document.root")
        .expect("document root");
    let service = ApplicationService::new(StoreInspectionBackend::new(Arc::clone(&store)))
        .with_project_service(projects);
    let grant = Capability {
        scope: CapabilityScope::Custom(format!("project:{project}")),
        actions: BTreeSet::from([CapabilityAction::Inspect, CapabilityAction::Propose]),
        constraints: CapabilityConstraints::unrestricted(),
    };
    let client = Client::new(
        InProcessTransport::new(service).with_capabilities(CapabilityContext::with_grants([grant])),
    );
    let tree = client
        .inspect_document(
            project.to_string(),
            imported.snapshot.to_string(),
            document.entity.to_string(),
        )
        .expect("remote semantic document tree");
    assert_eq!(tree.document, document.entity.to_string());
    assert_eq!(tree.sections[0].blocks.len(), 1);
    assert_eq!(tree.styles[0].key, "RemoteBody");
    assert_eq!(tree.styles[0].name.as_deref(), Some("Remote Body"));
    let response = client
        .propose_document_paragraph_edit(
            project.to_string(),
            imported.snapshot.to_string(),
            paragraph.to_string(),
            "Proposed site area",
        )
        .expect("remote paragraph candidate");
    assert_eq!(response.project, project.to_string());
    assert_eq!(response.base_snapshot, imported.snapshot.to_string());
    assert_eq!(response.path, "report.docx");
    assert_eq!(response.validation, DocumentCandidateValidation::Pending);
    assert_eq!(std::fs::read(path).expect("live source"), original);

    drop(store);
    std::fs::remove_file(database).expect("remove database");
    std::fs::remove_dir_all(root).expect("remove project");
}

#[allow(
    clippy::too_many_lines,
    reason = "This transport integration test verifies the complete host grant, remote round trip, candidate, and shared-operation lifecycle."
)]
/// Verifies the CAD line candidate traverses the typed remote protocol boundary.
#[test]
fn cad_line_candidate_round_trip_uses_host_grant_and_shared_operation() {
    let root = std::env::temp_dir().join(format!("scriberig-remote-cad-{}", EntityId::new()));
    std::fs::create_dir_all(&root).expect("create project");
    let mut drawing = dxf::Drawing::new();
    drawing.add_layer(dxf::tables::Layer {
        name: "LABELS".to_owned(),
        ..Default::default()
    });
    drawing.add_entity(dxf::entities::Entity::new(dxf::entities::EntityType::Line(
        dxf::entities::Line {
            p1: dxf::Point::new(0.0, 0.0, 0.0),
            p2: dxf::Point::new(10.0, 0.0, 0.0),
            ..Default::default()
        },
    )));
    let mut original = Vec::new();
    drawing.save(&mut original).expect("serialize DXF");
    let path = root.join("drawing.dxf");
    std::fs::write(&path, &original).expect("write DXF");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let projects = scriberig_project::ProjectService::new(Arc::clone(&store));
    let project = projects.scan(None, &root).expect("scan project").project.id;
    let imported = projects.import(project).expect("import DXF");
    let entity = store
        .project_entities(project)
        .expect("entity ownership")
        .into_iter()
        .find(|owned| owned.kind == "cad.entity")
        .expect("semantic line");
    let service = ApplicationService::new(StoreInspectionBackend::new(Arc::clone(&store)))
        .with_project_service(projects);
    let grant = Capability {
        scope: CapabilityScope::Custom(format!("project:{project}")),
        actions: BTreeSet::from([CapabilityAction::Inspect, CapabilityAction::Propose]),
        constraints: CapabilityConstraints::unrestricted(),
    };
    let client = Client::new(
        InProcessTransport::new(service).with_capabilities(CapabilityContext::with_grants([grant])),
    );
    let drawing = store
        .project_entities(project)
        .expect("entity ownership")
        .into_iter()
        .find(|owned| owned.kind == "cad.drawing")
        .expect("drawing");
    let summary = client
        .inspect_cad_drawing(
            project.to_string(),
            imported.snapshot.to_string(),
            drawing.entity.to_string(),
        )
        .expect("remote CAD drawing summary");
    assert_eq!(summary.entity_count, 1);
    assert_eq!(summary.layers.len(), 2);
    let entity_page = client
        .cad_entities(
            project.to_string(),
            imported.snapshot.to_string(),
            drawing.entity.to_string(),
            0,
            1,
        )
        .expect("remote CAD entity page");
    assert_eq!(entity_page.entities.len(), 1);
    assert_eq!(entity_page.total, 1);
    let bounded = client
        .cad_entities_intersecting_bounds(CadBoundsQueryRequest {
            project: project.to_string(),
            snapshot: imported.snapshot.to_string(),
            drawing: drawing.entity.to_string(),
            space: CadDrawingSpace::Model,
            bounds: CadBounds2D {
                min_x: "9".into(),
                min_y: "-1".into(),
                max_x: "11".into(),
                max_y: "1".into(),
            },
            offset: 0,
            limit: 10,
        })
        .expect("remote CAD bounds query");
    assert_eq!(bounded.total, 1);
    assert_eq!(bounded.entities[0].id, entity.entity.to_string());
    assert!(bounded.complete);
    let response = client
        .propose_cad_line_edit(
            project.to_string(),
            imported.snapshot.to_string(),
            entity.entity.to_string(),
            CadCoordinate {
                x: "1".into(),
                y: "2".into(),
                z: "0".into(),
            },
            CadCoordinate {
                x: "11".into(),
                y: "12".into(),
                z: "0".into(),
            },
        )
        .expect("remote CAD candidate");
    assert_eq!(response.project, project.to_string());
    assert_eq!(response.base_snapshot, imported.snapshot.to_string());
    assert_eq!(response.path, "drawing.dxf");
    assert_eq!(response.validation, CadCandidateValidation::Pending);
    let destination_layer = summary
        .layers
        .iter()
        .find(|layer| layer.name == "LABELS")
        .expect("destination layer")
        .id
        .clone();
    let layer_response = client
        .propose_cad_layer_change(
            project.to_string(),
            imported.snapshot.to_string(),
            entity.entity.to_string(),
            destination_layer,
        )
        .expect("remote CAD layer candidate");
    assert_eq!(layer_response.project, project.to_string());
    assert_eq!(layer_response.base_snapshot, imported.snapshot.to_string());
    assert_eq!(layer_response.path, "drawing.dxf");
    assert_eq!(layer_response.validation, CadCandidateValidation::Pending);
    assert_eq!(std::fs::read(path).expect("live source"), original);
    drop(store);
    std::fs::remove_file(database).expect("remove database");
    std::fs::remove_dir_all(root).expect("remove project");
}
