//! Verifies stale DOCX edits cannot overwrite a newer semantic successor.

use std::fs;
use std::io::{Cursor, Read, Write};
use std::sync::Arc;

use scriberig_domain_document::{DocumentBlock, Paragraph};
use scriberig_project::{CandidateValidation, CandidateValidationProfile, ProjectService};
use scriberig_store::{RevisionStore, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;
use zip::write::SimpleFileOptions;

use crate::docx_project;

/// Rejects a validated paragraph candidate after another edit advances its base snapshot.
#[test]
fn docx_candidate_from_stale_snapshot_cannot_overwrite_successor() {
    let root = docx_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan project");
    let project = scan.project.id;
    let base = service.import(project).expect("import DOCX").snapshot;
    let paragraph = store
        .project_entities(project)
        .expect("project entities")
        .into_iter()
        .filter(|entity| entity.kind == "document.paragraph")
        .find_map(|entity| {
            let revision = store.resolve(base, entity.entity).ok()??;
            let revision = store.get(revision).ok()??;
            let value: Paragraph = serde_json::from_slice(revision.payload()).ok()?;
            (paragraph_text(&value) == "Existing site area").then_some(value.id)
        })
        .expect("target paragraph");
    let edit = |replacement: &str| {
        service
            .propose_docx_paragraph_edit(project, base, paragraph, replacement)
            .expect("propose exact-base paragraph edit")
    };
    let winner = edit("Proposed site area");
    let stale = edit("Stale site area");
    let validate = |candidate: &str| {
        service
            .validate_candidate_profile_cancellable(
                candidate,
                CandidateValidationProfile::DocxPackage,
                &std::sync::atomic::AtomicBool::new(false),
            )
            .expect("validate staged DOCX package")
    };
    let winner = validate(&winner.id);
    let stale = validate(&stale.id);
    assert_eq!(winner.validation, CandidateValidation::Passed);
    assert_eq!(stale.validation, CandidateValidation::Passed);

    let applied = service.apply_candidate(&winner).expect("apply winner");
    let current = store
        .project(project)
        .expect("project state")
        .expect("project")
        .current_snapshot;
    assert_eq!(current, Some(applied.successor_snapshot));
    assert!(service.apply_candidate(&stale).is_err());
    assert_eq!(
        store
            .project(project)
            .expect("project state after stale rejection")
            .expect("project")
            .current_snapshot,
        current,
        "stale DOCX candidate must not advance canonical history"
    );
    let materialized = fs::read(root.join("report.docx")).expect("winning materialization");
    let source = store
        .project_file_descriptors(project)
        .expect("materialized source descriptor")
        .into_iter()
        .find(|file| file.relative_path == "report.docx")
        .expect("DOCX descriptor");
    assert_eq!(source.digest, *blake3::hash(&materialized).as_bytes());
    let document_id = scriberig_domain_docx::semantic_document_id(source.entity);
    let reopened = scriberig_domain_docx::import_docx(&materialized, document_id)
        .expect("reopen winning document");
    let blocks = &reopened.document.sections[0].blocks;
    assert!(blocks.iter().any(|block| matches!(
        block,
        DocumentBlock::Paragraph(value) if paragraph_text(value) == "Proposed site area"
    )));
    assert!(!blocks.iter().any(|block| matches!(
        block,
        DocumentBlock::Paragraph(value) if paragraph_text(value) == "Stale site area"
    )));

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}

/// Keeps an external opaque package change when a validated semantic candidate is stale.
#[test]
fn external_opaque_docx_change_is_not_materialized_as_deletion() {
    let root = docx_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let project = service.scan(None, &root).expect("scan project").project.id;
    let base = service.import(project).expect("import DOCX").snapshot;
    let paragraph = store
        .project_entities(project)
        .expect("project entities")
        .into_iter()
        .filter(|entity| entity.kind == "document.paragraph")
        .find_map(|entity| {
            let revision = store.resolve(base, entity.entity).ok()??;
            let revision = store.get(revision).ok()??;
            let value: Paragraph = serde_json::from_slice(revision.payload()).ok()?;
            (paragraph_text(&value) == "Existing site area").then_some(value.id)
        })
        .expect("target paragraph");
    let candidate = service
        .propose_docx_paragraph_edit(project, base, paragraph, "Proposed site area")
        .expect("propose paragraph edit");
    let candidate = service
        .validate_candidate_profile_cancellable(
            &candidate.id,
            CandidateValidationProfile::DocxPackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("validate staged DOCX");
    assert_eq!(candidate.validation, CandidateValidation::Passed);

    let path = root.join("report.docx");
    let changed = replace_package_member(
        &fs::read(&path).expect("read original package"),
        "customXml/opaque.bin",
        b"external opaque update\0",
    );
    fs::write(&path, &changed).expect("write external opaque change");
    let error = service
        .apply_candidate(&candidate)
        .expect_err("stale candidate must not overwrite external opaque bytes");
    assert_eq!(error.code, "filesystem_conflict");
    assert_eq!(fs::read(&path).expect("external bytes remain"), changed);
    assert_eq!(
        store
            .project(project)
            .expect("project state")
            .expect("project")
            .current_snapshot,
        Some(base),
        "rejected opaque source change must not advance canonical history"
    );

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}

/// Rewrites one ZIP member while preserving the remaining package member contents.
///
/// - `package`: Exact original archive bytes.
/// - `target`: Package path whose bytes are replaced.
/// - `replacement`: New opaque member bytes.
///
/// # Returns
/// Repacked archive bytes with only the selected member content changed.
fn replace_package_member(package: &[u8], target: &str, replacement: &[u8]) -> Vec<u8> {
    let mut reader = zip::ZipArchive::new(Cursor::new(package)).expect("open source package");
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..reader.len() {
        let mut member = reader.by_index(index).expect("read package member");
        let name = member.name().to_owned();
        let mut bytes = Vec::new();
        member.read_to_end(&mut bytes).expect("read member bytes");
        writer
            .start_file(name.clone(), SimpleFileOptions::default())
            .expect("start replacement package member");
        writer
            .write_all(if name == target { replacement } else { &bytes })
            .expect("write package member");
    }
    writer
        .finish()
        .expect("finish updated package")
        .into_inner()
}

/// Joins the text runs in one imported paragraph.
///
/// # Parameters
/// - `paragraph`: Semantic paragraph whose authored text is read.
///
/// # Returns
/// The exact concatenated run text.
fn paragraph_text(paragraph: &Paragraph) -> String {
    paragraph.runs.iter().map(|run| run.text.as_str()).collect()
}
