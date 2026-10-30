//! Verifies exact-baseline DOCX paragraph reconciliation and fail-closed cases.

use std::io::{Cursor, Write};

use scriberig_core::EntityId;
use scriberig_domain_document::DocumentBlock;
use scriberig_domain_docx::{
    DocxReconciliationOutcome, import_docx, materialize_reconciled_docx, reconcile_external_docx,
    replace_simple_paragraph,
};
use zip::write::SimpleFileOptions;

/// Creates a minimal DOCX package containing two paragraphs.
///
/// # Parameters
/// - `first`: First paragraph text.
/// - `second`: Second paragraph text.
///
/// # Returns
/// Exact ZIP package bytes for the supplied paragraph texts.
fn fixture(first: &str, second: &str) -> Vec<u8> {
    let xml = format!(
        r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>{first}</w:t></w:r></w:p><w:p><w:r><w:t>{second}</w:t></w:r></w:p></w:body></w:document>"#
    );
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    archive
        .start_file("word/document.xml", SimpleFileOptions::default())
        .expect("document entry");
    archive.write_all(xml.as_bytes()).expect("document XML");
    archive.finish().expect("finish DOCX").into_inner()
}

/// Creates a DOCX with persistent paragraph identities in the requested order.
///
/// # Parameters
/// - `paragraphs`: Persistent paragraph IDs and their authored text.
///
/// # Returns
/// DOCX bytes whose paragraph identities do not depend on sequence position.
fn stable_fixture(paragraphs: &[(&str, &str)]) -> Vec<u8> {
    let mut content = String::new();
    for (id, text) in paragraphs {
        use std::fmt::Write as _;
        write!(
            content,
            r#"<w:p w14:paraId="{id}"><w:r><w:t>{text}</w:t></w:r></w:p>"#
        )
        .expect("write to string");
    }
    let xml = format!(
        r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"><w:body>{content}</w:body></w:document>"#
    );
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    archive
        .start_file("word/document.xml", SimpleFileOptions::default())
        .expect("document entry");
    archive.write_all(xml.as_bytes()).expect("document XML");
    archive.finish().expect("finish DOCX").into_inner()
}

/// Retrieves mutable text from a paragraph at its stable block index.
///
/// # Parameters
/// - `document`: Semantic document to modify.
/// - `index`: Paragraph block index.
/// - `text`: Replacement text.
fn set_text(document: &mut scriberig_domain_document::Document, index: usize, text: &str) {
    let DocumentBlock::Paragraph(paragraph) = &mut document.sections[0].blocks[index] else {
        panic!("expected paragraph block");
    };
    text.clone_into(&mut paragraph.runs[0].text);
}

/// Merges independent external and canonical paragraph changes.
#[test]
fn merges_disjoint_paragraph_edits_against_exact_baseline() {
    let baseline = fixture("First", "Second");
    let external = fixture("External first", "Second");
    let id = EntityId::new();
    let mut current = import_docx(&baseline, id)
        .expect("baseline import")
        .document;
    set_text(&mut current, 1, "Canonical second");

    let outcome = reconcile_external_docx(
        &baseline,
        *blake3::hash(&baseline).as_bytes(),
        &current,
        &external,
        id,
    );
    let DocxReconciliationOutcome::Merged(merged) = outcome else {
        panic!("independent changes should merge");
    };
    assert_eq!(
        match &merged.sections[0].blocks[0] {
            DocumentBlock::Paragraph(paragraph) => paragraph.runs[0].text.as_str(),
            _ => panic!("expected paragraph"),
        },
        "External first"
    );
    assert_eq!(
        match &merged.sections[0].blocks[1] {
            DocumentBlock::Paragraph(paragraph) => paragraph.runs[0].text.as_str(),
            _ => panic!("expected paragraph"),
        },
        "Canonical second"
    );
}

/// Reports conflicting edits to the same stable paragraph identity.
#[test]
fn same_paragraph_edits_conflict() {
    let baseline = fixture("First", "Second");
    let external = fixture("External", "Second");
    let id = EntityId::new();
    let mut current = import_docx(&baseline, id)
        .expect("baseline import")
        .document;
    set_text(&mut current, 0, "Canonical");
    let outcome = reconcile_external_docx(
        &baseline,
        *blake3::hash(&baseline).as_bytes(),
        &current,
        &external,
        id,
    );
    assert!(matches!(outcome, DocxReconciliationOutcome::Conflict(ids) if ids.len() == 1));
}

/// Rejects stale baselines and structure changes without inventing identity.
#[test]
fn stale_baseline_and_structural_change_are_ambiguous() {
    let baseline = fixture("First", "Second");
    let id = EntityId::new();
    let current = import_docx(&baseline, id)
        .expect("baseline import")
        .document;
    let external = fixture("First", "Second changed");
    assert!(matches!(
        reconcile_external_docx(&baseline, [0; 32], &current, &external, id),
        DocxReconciliationOutcome::Ambiguous(_)
    ));

    let inserted = fixture("Inserted", "First");
    assert!(matches!(
        reconcile_external_docx(
            &baseline,
            *blake3::hash(&baseline).as_bytes(),
            &current,
            &inserted,
            id
        ),
        DocxReconciliationOutcome::Ambiguous(_)
    ));
}

/// Applies a merged canonical paragraph change over external package bytes.
#[test]
fn materializes_disjoint_merge_into_external_package() {
    let baseline = fixture("First", "Second");
    let id = EntityId::new();
    let mut current = import_docx(&baseline, id)
        .expect("baseline import")
        .document;
    set_text(&mut current, 0, "Canonical first");
    let DocumentBlock::Paragraph(external_target) = &current.sections[0].blocks[1] else {
        panic!("expected second paragraph");
    };
    let external = replace_simple_paragraph(&baseline, id, external_target.id, "External second")
        .expect("external paragraph edit");
    let merged = reconcile_external_docx(
        &baseline,
        *blake3::hash(&baseline).as_bytes(),
        &current,
        &external,
        id,
    );
    let DocxReconciliationOutcome::Merged(merged) = merged else {
        panic!("independent edits should merge");
    };

    let output = materialize_reconciled_docx(&external, &merged)
        .expect("materialize canonical change over external package");
    let reopened = import_docx(&output, id).expect("reopen merged package");
    assert_eq!(reopened.document, merged);
    assert_ne!(blake3::hash(&output), blake3::hash(&external));
}

/// Uses Word's persistent paragraph IDs and reports reordered identity as conflict.
#[test]
fn persistent_paragraph_ids_survive_text_edits_and_detect_reordered_identity() {
    let baseline = stable_fixture(&[("00000001", "First"), ("00000002", "Second")]);
    let id = EntityId::new();
    let imported = import_docx(&baseline, id).expect("import stable paragraph IDs");
    assert_eq!(imported.stable_paragraph_ids.len(), 2);
    let first_id = match &imported.document.sections[0].blocks[0] {
        DocumentBlock::Paragraph(paragraph) => paragraph.id,
        _ => panic!("expected first paragraph"),
    };
    let external = stable_fixture(&[("00000002", "Moved and edited"), ("00000001", "First")]);
    let outcome = reconcile_external_docx(
        &baseline,
        *blake3::hash(&baseline).as_bytes(),
        &imported.document,
        &external,
        id,
    );
    assert!(matches!(
        outcome,
        DocxReconciliationOutcome::Conflict(ids) if ids.contains(&first_id)
    ));
}

/// Rejects duplicate persistent paragraph identities before semantic import.
#[test]
fn duplicate_persistent_paragraph_ids_are_rejected() {
    let bytes = stable_fixture(&[("00000001", "First"), ("00000001", "Second")]);
    assert!(import_docx(&bytes, EntityId::new()).is_err());
}

/// Rejects multiple positional paragraph edits without persistent source IDs.
#[test]
fn multiple_changed_positional_paragraphs_are_ambiguous() {
    let baseline = fixture("First", "Second");
    let external = fixture("External first", "External second");
    let id = EntityId::new();
    let current = import_docx(&baseline, id)
        .expect("import baseline")
        .document;

    assert!(matches!(
        reconcile_external_docx(
            &baseline,
            *blake3::hash(&baseline).as_bytes(),
            &current,
            &external,
            id
        ),
        DocxReconciliationOutcome::Ambiguous(_)
    ));
}
