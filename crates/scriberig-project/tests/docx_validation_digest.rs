//! Verifies DOCX validation evidence is bound to exact durable candidate bytes.

use std::fs;
use std::io::{Cursor, Write};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use scriberig_project::{CandidateValidation, CandidateValidationProfile, ProjectService};
use scriberig_store_sqlite::SqliteStore;
use zip::write::SimpleFileOptions;

/// Creates a fresh project containing one safely editable DOCX paragraph.
///
/// # Returns
/// Temporary project directory with its source document.
fn create_project() -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "scriberig-docx-validation-{}",
        scriberig_core::EntityId::new()
    ));
    fs::create_dir_all(&root).expect("create project directory");
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    archive
        .start_file("word/document.xml", SimpleFileOptions::default())
        .expect("document entry");
    archive
        .write_all(br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Original paragraph</w:t></w:r></w:p></w:body></w:document>"#)
        .expect("document XML");
    fs::write(
        root.join("report.docx"),
        archive.finish().expect("finish package").into_inner(),
    )
    .expect("write source package");
    root
}

/// Rejects apply when durable DOCX output changes after successful validation.
#[test]
fn changed_docx_output_fails_exact_validation_digest_check() {
    let root = create_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open database"));
    let service = ProjectService::new(Arc::clone(&store));
    let project = service.scan(None, &root).expect("scan project").project.id;
    let imported = service.import(project).expect("import DOCX");
    let paragraph = store
        .project_entities(project)
        .expect("project entities")
        .into_iter()
        .find(|entity| entity.kind == "document.paragraph")
        .expect("semantic paragraph");
    let live_bytes = fs::read(root.join("report.docx")).expect("live package");
    let candidate = service
        .propose_docx_paragraph_edit(
            project,
            imported.snapshot,
            paragraph.entity,
            "Validated paragraph",
        )
        .expect("propose paragraph edit");
    let validated = service
        .validate_candidate_profile_cancellable(
            &candidate.id,
            CandidateValidationProfile::DocxPackage,
            &AtomicBool::new(false),
        )
        .expect("validate exact output");
    assert!(validated.validation_digest.is_some());

    let mut durable = store
        .project_candidate(&candidate.id)
        .expect("read candidate")
        .expect("durable candidate");
    durable.new.push(0);
    durable.effects[0]
        .new
        .as_mut()
        .expect("candidate effect output")
        .push(0);
    store
        .put_project_candidate(&durable)
        .expect("persist changed candidate output");
    let changed = service
        .candidate(&candidate.id)
        .expect("reload changed candidate")
        .expect("candidate remains available");
    let error = service
        .apply_candidate(&changed)
        .expect_err("changed output must require revalidation");

    assert_eq!(error.code, "validation_digest_mismatch");
    assert_eq!(
        fs::read(root.join("report.docx")).expect("live DOCX"),
        live_bytes
    );
    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}

/// Reports DOCX semantic validation failures against the persisted paragraph handle.
#[test]
fn docx_semantic_validation_diagnostic_names_the_target_paragraph() {
    let root = create_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open database"));
    let service = ProjectService::new(Arc::clone(&store));
    let project = service.scan(None, &root).expect("scan project").project.id;
    let imported = service.import(project).expect("import DOCX");
    let paragraph = store
        .project_entities(project)
        .expect("project entities")
        .into_iter()
        .find(|entity| entity.kind == "document.paragraph")
        .expect("semantic paragraph");
    let candidate = service
        .propose_docx_paragraph_edit(
            project,
            imported.snapshot,
            paragraph.entity,
            "Validated paragraph",
        )
        .expect("propose paragraph edit");
    let mut durable = store
        .project_candidate(&candidate.id)
        .expect("read candidate")
        .expect("durable candidate");
    let unchanged_bytes = durable.old.clone();
    durable.new.clone_from(&unchanged_bytes);
    durable.effects[0].new = Some(unchanged_bytes);
    store
        .put_project_candidate(&durable)
        .expect("persist semantically unchanged candidate output");

    let validated = service
        .validate_candidate_profile_cancellable(
            &candidate.id,
            CandidateValidationProfile::DocxPackage,
            &AtomicBool::new(false),
        )
        .expect("validation result");
    let CandidateValidation::Failed { diagnostics } = validated.validation else {
        panic!("semantic no-op must fail DOCX candidate validation");
    };
    assert!(
        diagnostics
            .join("\n")
            .contains(&format!("document.paragraph {}", paragraph.entity))
    );
    assert_eq!(validated.semantic_target, Some(paragraph.entity));

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}
