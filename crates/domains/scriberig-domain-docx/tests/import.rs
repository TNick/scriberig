//! Verifies DOCX imports expose document structure and probe exact write capability.

use std::io::{Cursor, Write};

use scriberig_core::EntityId;
use scriberig_domain_docx::import_docx;
use zip::write::SimpleFileOptions;

/// Creates a minimal package containing two paragraphs and one table cell.
///
/// # Returns
/// A generated DOCX package containing the document structure.
fn docx_fixture() -> Vec<u8> {
    package_with_document_xml(
        br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Existing site area</w:t></w:r></w:p><w:p><w:r><w:t>Preserve this paragraph</w:t></w:r></w:p><w:tbl><w:tr><w:tc><w:p><w:r><w:t>parcel 124</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:body></w:document>"#,
    )
}

/// Packages caller-supplied main document XML as a minimal DOCX archive.
///
/// # Parameters
/// - `xml`: `WordprocessingML` document payload.
///
/// # Returns
/// ZIP package bytes with the main document part.
fn package_with_document_xml(xml: &[u8]) -> Vec<u8> {
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    archive
        .start_file("word/document.xml", SimpleFileOptions::default())
        .expect("document entry");
    archive.write_all(xml).expect("document XML");
    archive.finish().expect("finish DOCX fixture").into_inner()
}

/// Keeps pagination markers out of canonical document identity and structure.
#[test]
fn rendered_page_break_does_not_change_semantic_document_structure() {
    let without_page_marker = import_docx(&docx_fixture(), EntityId::new())
        .expect("import source without rendered page marker");
    let with_page_marker = package_with_document_xml(
        br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Existing site area</w:t><w:lastRenderedPageBreak/></w:r></w:p><w:p><w:r><w:t>Preserve this paragraph</w:t></w:r></w:p><w:tbl><w:tr><w:tc><w:p><w:r><w:t>parcel 124</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:body></w:document>"#,
    );
    let with_page_marker = import_docx(&with_page_marker, without_page_marker.document.id)
        .expect("import source with rendered page marker");

    assert_eq!(with_page_marker.document, without_page_marker.document);
    assert_ne!(
        with_page_marker.source_digest,
        without_page_marker.source_digest
    );
}

/// Imports paragraph and table structure while preserving exact package evidence.
#[test]
fn imports_docx_content_and_probes_safe_paragraph_capability() {
    let bytes = docx_fixture();
    let id = EntityId::new();
    let imported = import_docx(&bytes, id).expect("import minimal DOCX");
    assert_eq!(imported.document.id, id);
    assert_eq!(imported.source_digest, *blake3::hash(&bytes).as_bytes());
    assert!(imported.editable);
    assert!(imported.diagnostic.is_some());
    assert_eq!(imported.document.sections[0].blocks.len(), 3);
    assert!(matches!(
        &imported.document.sections[0].blocks[0],
        scriberig_domain_document::DocumentBlock::Paragraph(paragraph)
            if paragraph.runs[0].text == "Existing site area"
    ));
    assert!(matches!(
        &imported.document.sections[0].blocks[2],
        scriberig_domain_document::DocumentBlock::Table(table)
            if table.rows[0].cells[0].id != id
    ));
}

/// Imports the style catalog as typed extensions while retaining paragraph references.
#[test]
fn imports_named_styles_as_typed_document_extensions() {
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    archive
        .start_file("word/document.xml", SimpleFileOptions::default())
        .expect("document entry");
    archive
        .write_all(br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:pPr><w:pStyle w:val="BodyText"/></w:pPr><w:r><w:t>Authored text</w:t></w:r></w:p></w:body></w:document>"#)
        .expect("document XML");
    archive
        .start_file("word/styles.xml", SimpleFileOptions::default())
        .expect("styles entry");
    archive
        .write_all(br#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:styleId="BodyText"><w:name w:val="Body Text"/></w:style></w:styles>"#)
        .expect("styles XML");
    let bytes = archive.finish().expect("finish DOCX").into_inner();

    let imported = import_docx(&bytes, EntityId::new()).expect("import styled document");
    assert!(matches!(
        &imported.document.extensions[..],
        [scriberig_domain_document::DocumentExtension::Style(style)]
            if style.key == "BodyText" && style.name.as_deref() == Some("Body Text")
    ));
    let scriberig_domain_document::DocumentBlock::Paragraph(paragraph) =
        &imported.document.sections[0].blocks[0]
    else {
        panic!("expected paragraph");
    };
    assert_eq!(
        paragraph.style.as_ref().map(|style| style.key.as_str()),
        Some("BodyText")
    );
    assert_eq!(paragraph.runs[0].text, "Authored text");
}

/// Resolves hyperlink relationship IDs into explicit semantic run destinations.
#[test]
fn imports_external_hyperlink_destinations_on_inline_runs() {
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    archive
        .start_file("word/document.xml", SimpleFileOptions::default())
        .expect("document entry");
    archive
        .write_all(br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:body><w:p><w:r><w:t>Read </w:t></w:r><w:hyperlink r:id="rId1"><w:r><w:t>the report</w:t></w:r></w:hyperlink></w:p></w:body></w:document>"#)
        .expect("document XML");
    archive
        .start_file("word/_rels/document.xml.rels", SimpleFileOptions::default())
        .expect("relationships entry");
    archive
        .write_all(br#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.test/report" TargetMode="External"/></Relationships>"#)
        .expect("relationship XML");
    let bytes = archive.finish().expect("finish DOCX").into_inner();

    let imported = import_docx(&bytes, EntityId::new()).expect("import hyperlink document");
    assert!(!imported.editable);
    assert!(imported.diagnostic.is_some());
    let scriberig_domain_document::DocumentBlock::Paragraph(paragraph) =
        &imported.document.sections[0].blocks[0]
    else {
        panic!("expected paragraph");
    };
    assert_eq!(paragraph.runs[0].text, "Read ");
    assert_eq!(paragraph.runs[0].hyperlink, None);
    assert_eq!(paragraph.runs[1].text, "the report");
    assert_eq!(
        paragraph.runs[1].hyperlink.as_deref(),
        Some("https://example.test/report")
    );
}
