//! Verifies semantic paragraph insertion uses exact section and candidate semantics.

use std::fs;
use std::sync::Arc;

use scriberig_domain_document::{DocumentBlock, Paragraph};
use scriberig_project::{CandidateValidation, CandidateValidationProfile, ProjectService};
use scriberig_store::SnapshotStore;
use scriberig_store_sqlite::SqliteStore;

use crate::docx_project;

/// Appends one paragraph while preserving existing semantic block identities.
#[test]
fn docx_paragraph_insert_updates_section_without_revising_existing_paragraphs() {
    let root = docx_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan project");
    let project = scan.project.id;
    let base = service.import(project).expect("import DOCX").snapshot;
    let section = store
        .project_entities(project)
        .expect("project entities")
        .into_iter()
        .find(|entity| entity.kind == "document.section")
        .expect("document section");
    let section_revision = store
        .resolve(base, section.entity)
        .expect("resolve base section")
        .expect("section exists at base");
    let paragraph_revisions = store
        .project_entities(project)
        .expect("paragraph ownership")
        .into_iter()
        .filter(|entity| entity.kind == "document.paragraph")
        .map(|entity| {
            let revision = store
                .resolve(base, entity.entity)
                .expect("resolve paragraph")
                .expect("paragraph is in base");
            (entity.entity, revision)
        })
        .collect::<Vec<_>>();
    let original = fs::read(root.join("report.docx")).expect("original DOCX");
    let candidate = service
        .propose_docx_paragraph_insert(project, base, section.entity, "New closing paragraph")
        .expect("propose section append");
    assert_eq!(candidate.base_snapshot, base);
    assert_eq!(
        candidate.semantic_operation.as_deref(),
        Some("document.insert_paragraph")
    );
    assert_eq!(
        fs::read(root.join("report.docx")).expect("live source before apply"),
        original
    );

    let validated = service
        .validate_candidate_profile_cancellable(
            &candidate.id,
            CandidateValidationProfile::DocxPackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("validate staged DOCX");
    assert_eq!(validated.validation, CandidateValidation::Passed);
    let applied = service
        .apply_candidate(&validated)
        .expect("apply inserted paragraph");
    assert_eq!(applied.base_snapshot, base);
    assert_ne!(
        store
            .resolve(applied.successor_snapshot, section.entity)
            .expect("resolve updated section")
            .expect("section remains present"),
        section_revision,
        "section collection revision must include the appended block"
    );
    for (paragraph, previous_revision) in paragraph_revisions {
        assert_eq!(
            store
                .resolve(applied.successor_snapshot, paragraph)
                .expect("resolve existing paragraph after insert")
                .expect("existing paragraph remains present"),
            previous_revision
        );
    }
    let materialized = fs::read(root.join("report.docx")).expect("materialized DOCX");
    let descriptor = store
        .project_file_descriptors(project)
        .expect("project source evidence")
        .into_iter()
        .find(|file| file.relative_path == "report.docx")
        .expect("DOCX descriptor");
    assert_eq!(descriptor.digest, *blake3::hash(&materialized).as_bytes());
    let document_id = scriberig_domain_docx::semantic_document_id(descriptor.entity);
    let reopened = scriberig_domain_docx::import_docx(&materialized, document_id)
        .expect("reopen inserted DOCX");
    let Some(DocumentBlock::Paragraph(last)) = reopened.document.sections[0].blocks.last() else {
        panic!("inserted section tail must be a paragraph");
    };
    assert_eq!(paragraph_text(last), "New closing paragraph");

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}

/// Joins all text runs from one semantic paragraph.
///
/// # Parameters
/// - `paragraph`: Paragraph whose authored text is inspected.
///
/// # Returns
/// Exact concatenated paragraph text.
fn paragraph_text(paragraph: &Paragraph) -> String {
    paragraph.runs.iter().map(|run| run.text.as_str()).collect()
}
