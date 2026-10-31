//! Verifies DOCX onboarding stores paragraph and table-cell entities separately.

use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;
use std::sync::Arc;

use scriberig_domain_document::{DocumentBlock, Paragraph};
use scriberig_project::{
    CandidateValidation, CandidateValidationProfile, ProjectService, ProjectSourceChange,
    ProjectSourceChangeKind,
};
use scriberig_store::{RevisionStore, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;
use zip::write::SimpleFileOptions;

#[path = "docx_onboarding/external_reconcile.rs"]
mod external_reconcile;
#[path = "docx_onboarding/failure.rs"]
mod failure;
#[path = "docx_onboarding/import_structure.rs"]
mod import_structure;
#[path = "docx_onboarding/insert.rs"]
mod insert;
#[path = "docx_onboarding/stale_candidate.rs"]
mod stale_candidate;
#[path = "docx_onboarding/table_cell_edit.rs"]
mod table_cell_edit;
#[path = "docx_onboarding/target_kind.rs"]
mod target_kind;

/// Creates a unique project containing a small two-paragraph DOCX.
///
/// # Returns
/// Path to the fresh project fixture.
fn docx_project() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "scriberig-docx-onboarding-{}",
        scriberig_core::EntityId::new()
    ));
    fs::create_dir_all(&root).expect("create project");
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    archive
        .start_file("word/document.xml", SimpleFileOptions::default())
        .expect("document entry");
    archive
        .write_all(
            br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Existing site area</w:t></w:r></w:p><w:p><w:r><w:t>Preserve this paragraph</w:t></w:r></w:p><w:tbl><w:tr><w:tc><w:p><w:r><w:t>parcel 124</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:body></w:document>"#,
        )
        .expect("document XML");
    archive
        .start_file("word/styles.xml", SimpleFileOptions::default())
        .expect("styles entry");
    archive
        .write_all(
            br#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:styleId="FixtureBody"><w:name w:val="Fixture Body"/></w:style></w:styles>"#,
        )
        .expect("styles XML");
    archive
        .start_file("customXml/opaque.bin", SimpleFileOptions::default())
        .expect("opaque package member");
    archive
        .write_all(b"opaque-preserved\0")
        .expect("opaque bytes");
    fs::write(
        root.join("report.docx"),
        archive.finish().expect("finish DOCX").into_inner(),
    )
    .expect("persist DOCX");
    root
}

/// Edits, validates, applies, and reopens one paragraph while preserving the package.
#[allow(clippy::too_many_lines)]
#[test]
fn docx_paragraph_candidate_validates_applies_and_preserves_other_content() {
    // Keep candidate validation, apply, reopen, and unrelated-content checks in
    // one transaction trace so they cover the same exact source baseline.
    let root = docx_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan project");
    let project = scan.project.id;
    let original = fs::read(root.join("report.docx")).expect("original DOCX");
    let imported = service.import(project).expect("import DOCX");
    let document_root = store
        .project_entities(project)
        .expect("document ownership")
        .into_iter()
        .find(|entity| entity.kind == "document.root")
        .expect("semantic document root");
    let document_root_revision = store
        .resolve(imported.snapshot, document_root.entity)
        .expect("document root revision lookup")
        .expect("document root revision");
    let document_root_payload = store
        .get(document_root_revision)
        .expect("load document root")
        .expect("stored document root");
    let document_root_value: serde_json::Value =
        serde_json::from_slice(document_root_payload.payload()).expect("decode document root");
    assert_eq!(
        document_root_value["extensions"][0]["Style"]["key"],
        "FixtureBody"
    );
    assert_eq!(
        service
            .document_tree(project, imported.snapshot, document_root.entity)
            .expect("typed document tree")
            .styles[0]
            .name
            .as_deref(),
        Some("Fixture Body")
    );
    let original_artifact = service
        .artifacts(project)
        .expect("original artifact manifest")
        .into_iter()
        .find(|artifact| artifact.source.adapter == "docx")
        .expect("DOCX artifact");
    let original_artifact_revision = store
        .resolve(imported.snapshot, original_artifact.id.entity())
        .expect("resolve original artifact")
        .expect("original artifact revision");
    let paragraph = store
        .project_entities(project)
        .expect("semantic ownership")
        .into_iter()
        .filter(|entity| entity.kind == "document.paragraph")
        .find_map(|entity| {
            let revision = store
                .resolve(imported.snapshot, entity.entity)
                .ok()
                .flatten()?;
            let revision = store.get(revision).ok().flatten()?;
            let paragraph: Paragraph = serde_json::from_slice(revision.payload()).ok()?;
            (paragraph
                .runs
                .iter()
                .map(|run| run.text.as_str())
                .collect::<String>()
                == "Existing site area")
                .then_some(paragraph)
        })
        .expect("target paragraph");
    let target_revision = store
        .resolve(imported.snapshot, paragraph.id)
        .expect("target revision lookup")
        .expect("target paragraph revision");
    let preserved_id = store
        .project_entities(project)
        .expect("paragraph ownership")
        .into_iter()
        .filter(|entity| entity.kind == "document.paragraph")
        .find_map(|entity| {
            let revision = store
                .resolve(imported.snapshot, entity.entity)
                .ok()
                .flatten()?;
            let revision = store.get(revision).ok().flatten()?;
            let value: Paragraph = serde_json::from_slice(revision.payload()).ok()?;
            (value
                .runs
                .iter()
                .map(|run| run.text.as_str())
                .collect::<String>()
                == "Preserve this paragraph")
                .then_some(value.id)
        })
        .expect("unrelated paragraph");
    let preserved_revision = store
        .resolve(imported.snapshot, preserved_id)
        .expect("unrelated revision lookup")
        .expect("unrelated paragraph revision");
    let candidate = service
        .propose_docx_paragraph_edit(
            project,
            imported.snapshot,
            paragraph.id,
            "Proposed site area",
        )
        .expect("paragraph candidate");
    assert_eq!(candidate.base_snapshot, imported.snapshot);
    assert_eq!(candidate.validation, CandidateValidation::Pending);
    assert_eq!(
        candidate.semantic_operation.as_deref(),
        Some("document.replace_paragraph")
    );
    assert_eq!(
        fs::read(root.join("report.docx")).expect("unchanged source"),
        original
    );
    let corrupt = service
        .propose_project_change(
            project,
            imported.snapshot,
            vec![ProjectSourceChange {
                path: "report.docx".into(),
                kind: ProjectSourceChangeKind::Update,
                content: Some(b"not a DOCX package".to_vec()),
            }],
        )
        .expect("corrupt package candidate");
    let rejected = service
        .validate_candidate_profile_cancellable(
            &corrupt.id,
            CandidateValidationProfile::DocxPackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("structured package validation failure");
    assert!(matches!(
        rejected.validation,
        CandidateValidation::Failed { .. }
    ));
    assert!(service.apply_candidate(&rejected).is_err());
    assert_eq!(
        fs::read(root.join("report.docx")).expect("live source after rejection"),
        original
    );
    let validated = service
        .validate_candidate_profile_cancellable(
            &candidate.id,
            CandidateValidationProfile::DocxPackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("DOCX package validation");
    assert_eq!(validated.validation, CandidateValidation::Passed);
    let applied = service
        .apply_candidate(&validated)
        .expect("apply DOCX candidate");
    assert_ne!(applied.successor_snapshot, imported.snapshot);
    assert_eq!(
        store
            .change_provenance(applied.successor_snapshot)
            .expect("read document snapshot provenance")
            .expect("applied document change")
            .semantic_operation
            .as_deref(),
        Some("document.replace_paragraph")
    );
    assert_ne!(
        store
            .resolve(applied.successor_snapshot, paragraph.id)
            .expect("updated target lookup")
            .expect("updated target paragraph"),
        target_revision
    );
    assert_eq!(
        store
            .resolve(applied.successor_snapshot, preserved_id)
            .expect("preserved revision lookup")
            .expect("preserved paragraph"),
        preserved_revision
    );
    let semantic_diff = store
        .diff_snapshot_revisions(imported.snapshot, applied.successor_snapshot)
        .expect("semantic document diff");
    let changed_entities = semantic_diff
        .iter()
        .map(|change| match change {
            scriberig_store::SnapshotDiff::Added { entity, .. }
            | scriberig_store::SnapshotDiff::Changed { entity, .. }
            | scriberig_store::SnapshotDiff::Removed { entity, .. } => *entity,
        })
        .collect::<Vec<_>>();
    assert!(changed_entities.contains(&paragraph.id));
    assert!(!changed_entities.contains(&preserved_id));
    let cell_paragraph = store
        .project_entities(project)
        .expect("table paragraph ownership")
        .into_iter()
        .filter(|entity| entity.kind == "document.paragraph")
        .find_map(|entity| {
            let revision = store
                .resolve(applied.successor_snapshot, entity.entity)
                .ok()
                .flatten()?;
            let revision = store.get(revision).ok().flatten()?;
            let value: Paragraph = serde_json::from_slice(revision.payload()).ok()?;
            (value
                .runs
                .iter()
                .map(|run| run.text.as_str())
                .collect::<String>()
                == "parcel 124")
                .then_some(value.id)
        })
        .expect("table cell paragraph");
    let cell_candidate = service
        .propose_docx_paragraph_edit(
            project,
            applied.successor_snapshot,
            cell_paragraph,
            "parcel 125",
        )
        .expect("table cell paragraph candidate");
    assert_eq!(cell_candidate.semantic_changes.len(), 2);
    assert_eq!(
        cell_candidate.semantic_changes[0].property,
        "paragraph.text"
    );
    assert_eq!(cell_candidate.semantic_changes[0].before, "parcel 124");
    assert_eq!(cell_candidate.semantic_changes[0].after, "parcel 125");
    let cell_change = &cell_candidate.semantic_changes[1];
    assert_eq!(cell_change.property, "table_cell.paragraph_text");
    assert_eq!(cell_change.before, "parcel 124");
    assert_eq!(cell_change.after, "parcel 125");
    assert_eq!(
        store
            .project_entity(project, cell_change.target)
            .expect("table cell semantic owner")
            .expect("table cell semantic entity")
            .kind,
        "document.table_cell"
    );
    let reloaded_cell_candidate = service
        .candidate(&cell_candidate.id)
        .expect("reload table cell candidate")
        .expect("persisted table cell candidate");
    assert_eq!(
        reloaded_cell_candidate.semantic_changes,
        cell_candidate.semantic_changes
    );
    let cell_validated = service
        .validate_candidate_profile_cancellable(
            &cell_candidate.id,
            CandidateValidationProfile::DocxPackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("table cell candidate validation");
    let applied = service
        .apply_candidate(&cell_validated)
        .expect("apply table cell paragraph candidate");
    assert_eq!(
        store
            .resolve(applied.successor_snapshot, preserved_id)
            .expect("preserved revision after cell edit")
            .expect("preserved paragraph after cell edit"),
        preserved_revision
    );
    let updated_artifact = service
        .artifacts(project)
        .expect("updated artifact manifest")
        .into_iter()
        .find(|artifact| artifact.source.adapter == "docx")
        .expect("updated DOCX artifact");
    assert_eq!(updated_artifact.id, original_artifact.id);
    assert_ne!(
        updated_artifact.members[0].digest,
        original_artifact.members[0].digest
    );
    let updated_artifact_revision = store
        .resolve(applied.successor_snapshot, updated_artifact.id.entity())
        .expect("resolve updated artifact")
        .expect("updated artifact revision");
    assert_ne!(updated_artifact_revision, original_artifact_revision);
    let materialized = fs::read(root.join("report.docx")).expect("materialized DOCX");
    let source = store
        .project_file_descriptors(project)
        .expect("updated source manifest")
        .into_iter()
        .find(|file| file.relative_path == "report.docx")
        .expect("DOCX evidence");
    assert_eq!(source.digest, *blake3::hash(&materialized).as_bytes());
    let document_id = scriberig_domain_docx::semantic_document_id(source.entity);
    let reopened = scriberig_domain_docx::import_docx(&materialized, document_id)
        .expect("reopen materialized DOCX");
    let text = reopened.document.sections[0]
        .blocks
        .iter()
        .filter_map(|block| match block {
            DocumentBlock::Paragraph(value) => Some(
                value
                    .runs
                    .iter()
                    .map(|run| run.text.as_str())
                    .collect::<String>(),
            ),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(text, ["Proposed site area", "Preserve this paragraph"]);
    let DocumentBlock::Table(table) = &reopened.document.sections[0].blocks[2] else {
        panic!("expected table");
    };
    let DocumentBlock::Paragraph(cell_paragraph) = &table.rows[0].cells[0].blocks[0] else {
        panic!("expected table cell paragraph");
    };
    assert_eq!(cell_paragraph.runs[0].text, "parcel 125");
    let mut archive = zip::ZipArchive::new(Cursor::new(materialized)).expect("open package");
    let mut opaque = Vec::new();
    archive
        .by_name("customXml/opaque.bin")
        .expect("opaque member remains")
        .read_to_end(&mut opaque)
        .expect("read opaque member");
    assert_eq!(opaque, b"opaque-preserved\0");

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}
