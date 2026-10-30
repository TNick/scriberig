//! Verifies one mixed project imports office, CAD, QGIS, and geospatial semantics.

use std::fs;
use std::sync::Arc;

use scriberig_domain_geo::GeoFieldDefinition;
use scriberig_domain_spreadsheet::CellId;
use scriberig_project::{
    ArtifactCapability, CandidateValidation, CandidateValidationProfile, OpaquePreservation,
    ProjectService,
};
use scriberig_store::{RevisionStore, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;

use super::{fixture_evidence, support::mixed_project};

/// Imports mixed rich artifacts while retaining exact files and separate semantics.
#[allow(clippy::too_many_lines)]
#[test]
fn mixed_rich_project_preserves_sources_and_imports_domain_entities() {
    // Keep cross-domain references and later edits in one evolving snapshot trace.
    let checked_in_xlsx = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/rich-project/budget.xlsx");
    let checked_in_xlsx_bytes = fs::read(&checked_in_xlsx).expect("checked-in XLSX fixture");
    let root = mixed_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan mixed project");
    assert_eq!(scan.files.len(), 12);
    fixture_evidence::assert_scan_adapters(&scan.files);
    let project = scan.project.id;
    let evidence = store
        .project_file_descriptors(project)
        .expect("persisted evidence");
    assert_eq!(evidence.len(), 12);
    fixture_evidence::assert_source_digests(&root, &evidence);
    service.import(project).expect("import mixed project");
    let entities = store.project_entities(project).expect("semantic entities");
    fixture_evidence::assert_rust_source_imported(&entities);
    for kind in [
        "rust.item",
        "spreadsheet.workbook",
        "spreadsheet.worksheet",
        "spreadsheet.cell",
        "document.root",
        "document.paragraph",
        "cad.drawing",
        "cad.entity",
        "qgis.project",
        "qgis.layer_reference",
        "geo.dataset",
        "geo.layer",
        "geo.field",
        "geo.feature",
    ] {
        assert!(
            entities.iter().any(|entity| entity.kind == kind),
            "missing {kind}"
        );
    }
    let artifacts = service.artifacts(project).expect("artifact summaries");
    assert_eq!(artifacts.len(), 9);
    let roads = artifacts
        .iter()
        .find(|artifact| artifact.logical_key == "shapefile:roads")
        .expect("grouped Shapefile artifact");
    assert_eq!(roads.members.len(), 4);
    assert_eq!(
        roads.fidelity.capability,
        ArtifactCapability::ReadOnlyDerived
    );
    assert_eq!(
        roads.fidelity.opaque_preservation,
        OpaquePreservation::Unproven
    );
    assert!(
        roads.fidelity.diagnostic.as_deref().is_some_and(
            |diagnostic| diagnostic.contains("compound materialization remains inspect-only")
        )
    );
    assert!(
        roads
            .members
            .iter()
            .all(|member| member.project_path.is_some())
    );
    let project_record = store
        .project(project)
        .expect("read project")
        .expect("project");
    let snapshot = project_record.current_snapshot.expect("imported snapshot");
    let roads_dataset = entities
        .iter()
        .find(|entity| entity.kind == "geo.dataset" && entity.relative_path == "roads.shp")
        .expect("Shapefile uses shared geospatial dataset semantics");
    let roads_layer = entities
        .iter()
        .find(|entity| entity.kind == "geo.layer" && entity.relative_path == "roads.shp")
        .expect("Shapefile layer semantics");
    let roads_feature = entities
        .iter()
        .find(|entity| entity.kind == "geo.feature" && entity.relative_path == "roads.shp")
        .expect("Shapefile feature semantics");
    let roads_feature_revision = store
        .resolve(snapshot, roads_feature.entity)
        .expect("resolve Shapefile feature")
        .expect("feature revision");
    let roads_feature_record = store
        .get(roads_feature_revision)
        .expect("read Shapefile feature")
        .expect("feature payload");
    let roads_feature_model: scriberig_domain_geo::Feature =
        serde_json::from_slice(roads_feature_record.payload()).expect("decode feature");
    assert_eq!(roads_feature_model.layer, roads_layer.entity);
    assert_eq!(roads_feature_model.native_id.as_deref(), Some("1"));
    assert_eq!(
        roads_feature_model.attributes.get("road_id"),
        Some(&scriberig_domain_geo::GeoValue::Number("42".to_owned()))
    );
    let roads_dataset_revision = store
        .resolve(snapshot, roads_dataset.entity)
        .expect("resolve Shapefile dataset")
        .expect("dataset revision");
    let roads_dataset_record = store
        .get(roads_dataset_revision)
        .expect("read Shapefile dataset")
        .expect("dataset payload");
    let roads_dataset_model: serde_json::Value =
        serde_json::from_slice(roads_dataset_record.payload()).expect("decode dataset summary");
    assert_eq!(
        roads_dataset_model["provider"],
        "scriberig-domain-geo-shapefile-2d-readonly-v1"
    );
    let qgis_project = service
        .qgis_project(project, "project.qgs", snapshot)
        .expect("exact-snapshot QGIS project");
    assert_eq!(qgis_project.snapshot, snapshot);
    assert_eq!(qgis_project.project.layers.len(), 2);
    assert_eq!(
        qgis_project.project.layers[0].layer,
        Some(
            entities
                .iter()
                .find(|entity| {
                    entity.kind == "geo.layer" && entity.relative_path == "parcels.gpkg"
                })
                .expect("linked GeoLayer")
                .entity
        )
    );
    let qgis_roads = qgis_project
        .project
        .layers
        .iter()
        .find(|layer| layer.name == "Roads")
        .expect("QGIS Shapefile reference");
    assert_eq!(qgis_roads.resolved_path.as_deref(), Some("roads.shp"));
    assert_eq!(qgis_roads.dataset, Some(roads_dataset.entity));
    assert_eq!(qgis_roads.layer, Some(roads_layer.entity));
    let original_xlsx = fs::read(root.join("budget.xlsx")).expect("original XLSX bytes");
    fixture_evidence::assert_original_xlsx(&original_xlsx);
    let original_docx = fs::read(root.join("report.docx")).expect("original DOCX bytes");
    let geo_dataset = entities
        .iter()
        .find(|entity| entity.kind == "geo.dataset" && entity.relative_path == "parcels.gpkg")
        .expect("GeoPackage dataset");
    let geo_layer = entities
        .iter()
        .find(|entity| entity.kind == "geo.layer" && entity.relative_path == "parcels.gpkg")
        .expect("GeoPackage layer");
    let geo_fields = entities
        .iter()
        .filter(|entity| entity.kind == "geo.field" && entity.relative_path == "parcels.gpkg")
        .collect::<Vec<_>>();
    assert_eq!(geo_fields.len(), 3);
    for owned_field in geo_fields {
        let revision = store
            .resolve(snapshot, owned_field.entity)
            .expect("resolve exact field snapshot")
            .expect("field semantic revision");
        let record = store
            .get(revision)
            .expect("load field revision")
            .expect("field revision record");
        assert_eq!(record.object_type().as_str(), "geo.field");
        let field: GeoFieldDefinition =
            serde_json::from_slice(record.payload()).expect("decode semantic field revision");
        assert_eq!(field.id, owned_field.entity);
        assert_eq!(field.layer, geo_layer.entity);
        assert_eq!(
            field.definition.name,
            owned_field.name.as_deref().expect("field name")
        );
        assert_eq!(
            scriberig_domain_geo::semantic_field_id(field.layer, &field.definition.name),
            owned_field.entity
        );
    }
    let gpkg_digest = evidence
        .iter()
        .find(|descriptor| descriptor.relative_path == "parcels.gpkg")
        .expect("persisted GeoPackage evidence")
        .digest;
    let feature_page = service
        .geopackage_feature_page(project, "parcels.gpkg", snapshot, geo_layer.entity, 0, 1)
        .expect("bounded persisted GeoPackage feature page");
    assert_eq!(feature_page.features.len(), 1);
    assert!(feature_page.has_more);
    assert_eq!(feature_page.features[0].native_id.as_deref(), Some("124"));
    let geo_catalog = service
        .geopackage_catalog(project, "parcels.gpkg", snapshot, 0, 1)
        .expect("bounded persisted GeoPackage layer catalog");
    assert_eq!(geo_catalog.snapshot, snapshot);
    assert_eq!(geo_catalog.dataset.id, geo_dataset.entity);
    assert_eq!(geo_catalog.source_digest, gpkg_digest);
    assert_eq!(geo_catalog.total, 1);
    assert!(!geo_catalog.has_more);
    assert_eq!(geo_catalog.layers[0].layer.id, geo_layer.entity);
    assert_eq!(geo_catalog.layers[0].layer.name, "parcels");
    assert_eq!(geo_catalog.layers[0].feature_count, 2);
    assert!(geo_catalog.layers[0].editable);
    let geo_revision = store
        .resolve(snapshot, geo_dataset.entity)
        .expect("resolve dataset summary")
        .expect("dataset summary revision");
    let geo_payload = store
        .get(geo_revision)
        .expect("read dataset summary")
        .expect("dataset summary payload");
    let geo_summary: serde_json::Value =
        serde_json::from_slice(geo_payload.payload()).expect("decode dataset summary");
    assert_eq!(
        geo_summary["provider"],
        "rusqlite-0.40.2-staged-geopackage-writer-v1"
    );
    assert_eq!(geo_summary["source_digest"], serde_json::json!(gpkg_digest));
    fixture_evidence::assert_qgis_references(
        &store,
        snapshot,
        &entities,
        geo_dataset.entity,
        geo_layer.entity,
    );

    let budget_sheet = fixture_evidence::worksheet_by_name(&store, &entities, snapshot, "Budget");
    let workbook = entities
        .iter()
        .find(|entity| {
            entity.kind == "spreadsheet.workbook" && entity.relative_path == "budget.xlsx"
        })
        .expect("Budget workbook");
    let spreadsheet_matches = service
        .find_spreadsheet_cells(project, snapshot, workbook.entity, "e", 0, 1)
        .expect("find matching workbook cells");
    assert_eq!(spreadsheet_matches.snapshot, snapshot);
    assert_eq!(spreadsheet_matches.total, 2);
    assert!(spreadsheet_matches.has_more);
    assert_eq!(spreadsheet_matches.matches[0].worksheet, budget_sheet);
    assert_eq!(
        spreadsheet_matches.matches[0].cell,
        CellId::new(workbook.entity, budget_sheet, 3, 3).entity()
    );
    assert_eq!(spreadsheet_matches.matches[0].authored_value, "existing");
    let next_spreadsheet_matches = service
        .find_spreadsheet_cells(project, snapshot, workbook.entity, "e", 1, 1)
        .expect("read next matching workbook page");
    assert_eq!(next_spreadsheet_matches.total, 2);
    assert!(!next_spreadsheet_matches.has_more);
    assert_eq!(
        next_spreadsheet_matches.matches[0].authored_value,
        "preserve me"
    );
    let spreadsheet_candidate = super::semantic_candidate_assertions::propose_formula_cell_edit(
        &service,
        project,
        snapshot,
        workbook.entity,
        budget_sheet,
    );
    super::semantic_candidate_assertions::assert_formula_cell_diff(
        &service,
        &spreadsheet_candidate,
    );
    fixture_evidence::assert_live_file(&root, "budget.xlsx", &original_xlsx);
    let spreadsheet_validated = service
        .validate_candidate_profile_cancellable(
            &spreadsheet_candidate.id,
            CandidateValidationProfile::XlsxPackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("validate spreadsheet candidate");
    assert_eq!(
        spreadsheet_validated.validation,
        CandidateValidation::Passed
    );
    let spreadsheet_applied = service
        .apply_candidate(&spreadsheet_validated)
        .expect("apply spreadsheet candidate");
    fixture_evidence::assert_formula_candidate_change(
        &service,
        &store,
        project,
        snapshot,
        spreadsheet_applied.successor_snapshot,
        budget_sheet,
    );

    let docx_paragraph = fixture_evidence::paragraph_by_text(
        &store,
        &entities,
        spreadsheet_applied.successor_snapshot,
        "Existing site area",
    );
    let document_candidate = service
        .propose_docx_paragraph_edit(
            project,
            spreadsheet_applied.successor_snapshot,
            docx_paragraph,
            "Proposed site area",
        )
        .expect("mixed-project document candidate");
    super::semantic_candidate_assertions::assert_paragraph_diff(
        &service,
        &document_candidate,
        spreadsheet_applied.successor_snapshot,
        docx_paragraph,
    );
    fixture_evidence::assert_live_file(&root, "report.docx", &original_docx);
    let document_validated = service
        .validate_candidate_profile_cancellable(
            &document_candidate.id,
            CandidateValidationProfile::DocxPackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("validate document candidate");
    assert_eq!(document_validated.validation, CandidateValidation::Passed);
    let document_applied = service
        .apply_candidate(&document_validated)
        .expect("apply document candidate");
    fixture_evidence::assert_document_paragraph_change(
        &service,
        &store,
        project,
        spreadsheet_applied.successor_snapshot,
        document_applied.successor_snapshot,
        docx_paragraph,
    );
    let final_files = store
        .project_file_descriptors(project)
        .expect("post-apply source evidence");
    for source in &final_files {
        if !matches!(source.relative_path.as_str(), "budget.xlsx" | "report.docx") {
            let original_source = evidence
                .iter()
                .find(|original| original.relative_path == source.relative_path)
                .expect("original source evidence");
            assert_eq!(
                source.digest, original_source.digest,
                "{} was preserved",
                source.relative_path
            );
        }
    }
    let materialized_docx = final_files
        .iter()
        .find(|source| source.relative_path == "report.docx")
        .expect("materialized DOCX");
    let materialized_xlsx = final_files
        .iter()
        .find(|source| source.relative_path == "budget.xlsx")
        .expect("materialized XLSX");
    fixture_evidence::assert_materialized_digests(&root, materialized_docx, materialized_xlsx);
    let reopened_xlsx = scriberig_domain_xlsx::import_xlsx(
        &fs::read(root.join(&materialized_xlsx.relative_path)).expect("materialized XLSX bytes"),
        scriberig_domain_xlsx::semantic_workbook_id(materialized_xlsx.entity),
    )
    .expect("reopen mixed-project XLSX");
    fixture_evidence::assert_materialized_xlsx(&reopened_xlsx);
    let reopened_docx = scriberig_domain_docx::import_docx(
        &fs::read(root.join(&materialized_docx.relative_path)).expect("materialized DOCX bytes"),
        scriberig_domain_docx::semantic_document_id(materialized_docx.entity),
    )
    .expect("reopen mixed-project DOCX");
    let scriberig_domain_document::DocumentBlock::Paragraph(paragraph) =
        &reopened_docx.document.sections[0].blocks[0]
    else {
        panic!("expected document paragraph");
    };
    assert_eq!(paragraph.runs[0].text, "Proposed site area");
    fixture_evidence::assert_docx_opaque_anchor(&root);
    assert_ne!(
        document_applied.successor_snapshot,
        spreadsheet_applied.successor_snapshot
    );
    assert_eq!(
        fs::read(&checked_in_xlsx).expect("checked-in XLSX fixture after test"),
        checked_in_xlsx_bytes,
        "candidate edits operate on the fresh project copy"
    );

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}
