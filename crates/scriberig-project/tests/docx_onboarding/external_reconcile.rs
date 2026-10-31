//! Verifies external DOCX edits pass through baseline-bound reconciliation.

use std::fs;
use std::io::{Cursor, Read, Write};
use std::sync::Arc;

use scriberig_domain_document::Paragraph;
use scriberig_project::{
    CandidateValidationProfile, DocumentBlockView, ProjectService, ProjectSourceObservation,
};
use scriberig_store::{RevisionStore, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

use crate::docx_project;

/// Imports a conflict-free external paragraph edit after exact baseline checking.
#[test]
fn external_docx_paragraph_edit_is_reconciled_before_project_import() {
    let root = docx_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let project = service.scan(None, &root).expect("scan").project.id;
    let baseline_snapshot = service.import(project).expect("initial import").snapshot;
    let paragraph = store
        .project_entities(project)
        .expect("project entities")
        .into_iter()
        .filter(|entity| entity.kind == "document.paragraph")
        .find_map(|entity| {
            let revision = store.resolve(baseline_snapshot, entity.entity).ok()??;
            let revision = store.get(revision).ok()??;
            let value: Paragraph = serde_json::from_slice(revision.payload()).ok()?;
            (value
                .runs
                .iter()
                .map(|run| run.text.as_str())
                .collect::<String>()
                == "Existing site area")
                .then_some(value.id)
        })
        .expect("target paragraph");
    let baseline = fs::read(root.join("report.docx")).expect("baseline package");
    let document = scriberig_domain_docx::semantic_document_id(
        store
            .project_file_descriptors(project)
            .expect("source descriptors")[0]
            .entity,
    );
    let external = scriberig_domain_docx::replace_simple_paragraph(
        &baseline,
        document,
        paragraph,
        "External site area",
    )
    .expect("external paragraph edit");
    fs::write(root.join("report.docx"), &external).expect("write external package");
    let digest = *blake3::hash(&external).as_bytes();

    let report = service
        .handle_source_observation(project, "report.docx", Some(digest))
        .expect("reconcile and import external DOCX");
    assert!(report.imported_snapshot.is_some());
    let imported = service
        .document_tree(
            project,
            report.imported_snapshot.expect("imported snapshot"),
            document,
        )
        .expect("read imported document");
    assert!(matches!(
        &imported.sections[0].blocks[0],
        DocumentBlockView::Paragraph { paragraph: value }
            if value.runs.iter().map(|run| run.text.as_str()).collect::<String>() == "External site area"
    ));

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project fixture");
}

/// Applies disjoint canonical/external edits through the durable shared candidate path.
#[test]
fn disjoint_external_docx_edit_materializes_reconciled_candidate() {
    let root = docx_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let project = service.scan(None, &root).expect("scan").project.id;
    let original_snapshot = service.import(project).expect("initial import").snapshot;
    let original_file = store
        .project_files(project)
        .expect("original files")
        .into_iter()
        .find(|file| file.relative_path == "report.docx")
        .expect("DOCX source");
    let document = scriberig_domain_docx::semantic_document_id(original_file.entity);
    let first = paragraph_id(&store, project, original_snapshot, "Existing site area");
    let second = paragraph_id(
        &store,
        project,
        original_snapshot,
        "Preserve this paragraph",
    );

    let canonical = service
        .propose_docx_paragraph_edit(project, original_snapshot, first, "Canonical site area")
        .expect("propose canonical edit");
    let canonical = service
        .validate_candidate_profile_cancellable(
            &canonical.id,
            CandidateValidationProfile::DocxPackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("validate canonical edit");
    let canonical = service
        .apply_candidate(&canonical)
        .expect("apply canonical edit");

    // Recreate the persisted source baseline while retaining the newer semantic
    // snapshot, modeling an external edit racing with canonical advancement.
    store
        .replace_project_files(project, std::slice::from_ref(&original_file))
        .expect("restore prior source baseline");
    let external = scriberig_domain_docx::replace_simple_paragraph(
        &original_file.bytes,
        document,
        second,
        "External second paragraph",
    )
    .expect("external paragraph edit");
    let external = replace_package_member(
        &external,
        "customXml/opaque.bin",
        b"external opaque update\0",
    );
    fs::write(root.join("report.docx"), &external).expect("write external package");
    let observed_digest = *blake3::hash(&external).as_bytes();

    let result = service
        .handle_source_observation(project, "report.docx", Some(observed_digest))
        .expect("merge and apply external observation");
    assert_eq!(result.observation, ProjectSourceObservation::ExternalChange);
    let merged_snapshot = result.imported_snapshot.expect("merged snapshot");
    let merged = service
        .document_tree(project, merged_snapshot, document)
        .expect("read merged document");
    assert!(has_text(&merged.sections[0].blocks, "Canonical site area"));
    assert!(has_text(
        &merged.sections[0].blocks,
        "External second paragraph"
    ));
    let materialized = fs::read(root.join("report.docx")).expect("merged DOCX bytes");
    let reopened = scriberig_domain_docx::import_docx(&materialized, document)
        .expect("reopen materialized merge");
    assert_eq!(
        reopened.source_digest,
        *blake3::hash(&materialized).as_bytes()
    );
    assert_eq!(
        package_member(&materialized, "customXml/opaque.bin"),
        b"external opaque update\0"
    );

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project fixture");
    assert_ne!(canonical.successor_snapshot, merged_snapshot);
}

/// Preserves canonical and external bytes when both sides change one paragraph.
#[test]
fn same_paragraph_external_conflict_does_not_import_or_overwrite() {
    let root = docx_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let project = service.scan(None, &root).expect("scan").project.id;
    let baseline_snapshot = service.import(project).expect("initial import").snapshot;
    let baseline_file = store
        .project_files(project)
        .expect("baseline files")
        .into_iter()
        .find(|file| file.relative_path == "report.docx")
        .expect("DOCX source");
    let document = scriberig_domain_docx::semantic_document_id(baseline_file.entity);
    let paragraph = paragraph_id(&store, project, baseline_snapshot, "Existing site area");
    let candidate = service
        .propose_docx_paragraph_edit(project, baseline_snapshot, paragraph, "Canonical edit")
        .expect("propose canonical edit");
    let candidate = service
        .validate_candidate_profile_cancellable(
            &candidate.id,
            CandidateValidationProfile::DocxPackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("validate canonical edit");
    let canonical = service
        .apply_candidate(&candidate)
        .expect("apply canonical edit");
    store
        .replace_project_files(project, std::slice::from_ref(&baseline_file))
        .expect("retain exact source baseline for race fixture");
    let external = scriberig_domain_docx::replace_simple_paragraph(
        &baseline_file.bytes,
        document,
        paragraph,
        "External conflicting edit",
    )
    .expect("external conflicting edit");
    fs::write(root.join("report.docx"), &external).expect("write external conflict");
    let error = service
        .handle_source_observation(
            project,
            "report.docx",
            Some(*blake3::hash(&external).as_bytes()),
        )
        .expect_err("same paragraph changes must conflict");

    assert_eq!(error.code, "docx_reconciliation_conflict");
    assert_eq!(
        service.workspace(project).expect("workspace").snapshot,
        canonical.successor_snapshot
    );
    assert_eq!(
        fs::read(root.join("report.docx")).expect("observed bytes"),
        external
    );

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project fixture");
}

/// Resolves an imported paragraph by exact snapshot and authored text.
///
/// # Parameters
/// - `store`: Project's immutable revision store.
/// - `project`: Project owning the paragraph.
/// - `snapshot`: Exact snapshot to read.
/// - `text`: Paragraph text to locate.
///
/// # Returns
/// Stable paragraph identity matching the requested text.
fn paragraph_id(
    store: &SqliteStore,
    project: scriberig_core::ProjectId,
    snapshot: scriberig_core::SnapshotId,
    text: &str,
) -> scriberig_core::EntityId {
    store
        .project_entities(project)
        .expect("project entities")
        .into_iter()
        .filter(|entity| entity.kind == "document.paragraph")
        .find_map(|entity| {
            let revision = store.resolve(snapshot, entity.entity).ok()??;
            let revision = store.get(revision).ok()??;
            let value: Paragraph = serde_json::from_slice(revision.payload()).ok()?;
            (value
                .runs
                .iter()
                .map(|run| run.text.as_str())
                .collect::<String>()
                == text)
                .then_some(value.id)
        })
        .expect("paragraph text")
}

/// Checks whether one document block tree contains a paragraph with exact text.
///
/// # Parameters
/// - `blocks`: Ordered semantic blocks to inspect.
/// - `text`: Exact paragraph text.
///
/// # Returns
/// True when a top-level or table-cell paragraph matches.
fn has_text(blocks: &[DocumentBlockView], text: &str) -> bool {
    blocks.iter().any(|block| match block {
        DocumentBlockView::Paragraph { paragraph } => {
            paragraph
                .runs
                .iter()
                .map(|run| run.text.as_str())
                .collect::<String>()
                == text
        }
        DocumentBlockView::Table { table } => table
            .rows
            .iter()
            .any(|row| row.cells.iter().any(|cell| has_text(&cell.blocks, text))),
    })
}

/// Repackages one member with replacement bytes while retaining other members.
///
/// # Parameters
/// - `package`: Exact source package bytes.
/// - `target`: Package member name to replace.
/// - `replacement`: New exact member bytes.
///
/// # Returns
/// Repacked DOCX package bytes.
fn replace_package_member(package: &[u8], target: &str, replacement: &[u8]) -> Vec<u8> {
    let mut reader = ZipArchive::new(Cursor::new(package)).expect("open package");
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..reader.len() {
        let mut member = reader.by_index(index).expect("read package member");
        let name = member.name().to_owned();
        let mut bytes = Vec::new();
        member.read_to_end(&mut bytes).expect("read member bytes");
        writer
            .start_file(&name, SimpleFileOptions::default())
            .expect("start package member");
        writer
            .write_all(if name == target { replacement } else { &bytes })
            .expect("write package member");
    }
    writer.finish().expect("finish package").into_inner()
}

/// Reads exact uncompressed bytes from one DOCX package member.
///
/// # Parameters
/// - `package`: Exact package bytes.
/// - `name`: Member path to read.
///
/// # Returns
/// Exact uncompressed member payload.
fn package_member(package: &[u8], name: &str) -> Vec<u8> {
    let mut archive = ZipArchive::new(Cursor::new(package)).expect("open package");
    let mut member = archive.by_name(name).expect("find package member");
    let mut bytes = Vec::new();
    member.read_to_end(&mut bytes).expect("read member");
    bytes
}
