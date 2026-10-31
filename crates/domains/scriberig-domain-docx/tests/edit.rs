//! Verifies safe DOCX paragraph edits preserve unrelated package members.

use std::io::{Cursor, Read, Write};

use scriberig_core::EntityId;
use scriberig_domain_docx::{
    append_simple_paragraph, import_docx, replace_simple_paragraph, semantic_document_id,
};
use zip::write::SimpleFileOptions;

/// Creates a package with one main document and one opaque preserved member.
///
/// # Parameters
/// - `xml`: Main `WordprocessingML` document content.
///
/// # Returns
/// Deterministic in-memory DOCX package bytes.
fn package(xml: &str) -> Vec<u8> {
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    archive
        .start_file("word/document.xml", SimpleFileOptions::default())
        .expect("main document entry");
    archive.write_all(xml.as_bytes()).expect("write XML");
    archive
        .start_file("customXml/opaque.bin", SimpleFileOptions::default())
        .expect("opaque entry");
    archive
        .write_all(b"opaque payload\0")
        .expect("write opaque data");
    archive
        .start_file("word/header1.xml", SimpleFileOptions::default())
        .expect("header entry");
    archive
        .write_all(b"<w:hdr>opaque header</w:hdr>")
        .expect("write opaque header");
    archive
        .start_file("word/footer1.xml", SimpleFileOptions::default())
        .expect("footer entry");
    archive
        .write_all(b"<w:ftr>opaque footer</w:ftr>")
        .expect("write opaque footer");
    archive
        .start_file("word/comments.xml", SimpleFileOptions::default())
        .expect("comments entry");
    archive
        .write_all(b"<w:comments>opaque comment</w:comments>")
        .expect("write opaque comments");
    archive
        .start_file("word/styles.xml", SimpleFileOptions::default())
        .expect("styles entry");
    archive
        .write_all(b"<w:styles xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:style w:styleId=\"BodyText\"><w:name w:val=\"Body Text\"/></w:style></w:styles>")
        .expect("write styles");
    archive.finish().expect("finish archive").into_inner()
}

/// Creates a DOCX package with exact raw archive comment metadata.
///
/// # Parameters
/// - `xml`: Main document XML content.
/// - `comment`: Raw ZIP archive comment bytes.
///
/// # Returns
/// DOCX package bytes carrying the supplied archive comment.
fn package_with_archive_comment(xml: &str, comment: &[u8]) -> Vec<u8> {
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    archive.set_raw_comment(comment.to_vec().into_boxed_slice());
    archive
        .start_file("word/document.xml", SimpleFileOptions::default())
        .expect("main document entry");
    archive.write_all(xml.as_bytes()).expect("write XML");
    archive.finish().expect("finish archive").into_inner()
}

/// Replaces one simple paragraph and retains unrelated DOCX package content.
#[test]
fn simple_paragraph_edit_preserves_other_text_and_opaque_parts() {
    let original = package(
        r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:pPr><w:pStyle w:val="BodyText"/></w:pPr><w:r><w:t>Existing site area</w:t></w:r></w:p><w:p><w:pPr><w:pStyle w:val="BodyText"/></w:pPr><w:bookmarkStart w:id="1" w:name="preserve-anchor"/><w:r><w:t>Preserve this paragraph</w:t></w:r><w:bookmarkEnd w:id="1"/></w:p></w:body></w:document>"#,
    );
    let source = EntityId::from_bytes([4; 16]);
    let document_id = semantic_document_id(source);
    let imported = import_docx(&original, document_id).expect("import original");
    let scriberig_domain_document::DocumentBlock::Paragraph(paragraph) =
        &imported.document.sections[0].blocks[0]
    else {
        panic!("expected paragraph");
    };
    let updated = replace_simple_paragraph(
        &original,
        document_id,
        paragraph.id,
        "Proposed site & access",
    )
    .expect("safe paragraph edit");
    validate_members(&updated);
    let reopened = import_docx(&updated, document_id).expect("reopen updated document");
    let texts = reopened.document.sections[0]
        .blocks
        .iter()
        .filter_map(|block| match block {
            scriberig_domain_document::DocumentBlock::Paragraph(value) => Some(
                value
                    .runs
                    .iter()
                    .map(|run| run.text.as_str())
                    .collect::<String>(),
            ),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(texts, ["Proposed site & access", "Preserve this paragraph"]);
}

/// Preserves raw ZIP archive comment bytes during a semantic paragraph edit.
#[test]
fn paragraph_edit_preserves_raw_archive_comment() {
    let source = EntityId::from_bytes([9; 16]);
    let document_id = semantic_document_id(source);
    let comment = [0x53, 0x52, 0x00, 0xFF];
    let original = package_with_archive_comment(
        r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Original</w:t></w:r></w:p></w:body></w:document>"#,
        &comment,
    );
    let imported = import_docx(&original, document_id).expect("import commented DOCX");
    assert!(
        imported.editable,
        "commented package should remain editable"
    );
    let scriberig_domain_document::DocumentBlock::Paragraph(paragraph) =
        &imported.document.sections[0].blocks[0]
    else {
        panic!("expected paragraph");
    };

    let output = replace_simple_paragraph(&original, document_id, paragraph.id, "Updated")
        .expect("edit commented package");
    let source_archive = zip::ZipArchive::new(Cursor::new(original)).expect("source archive");
    let output_archive = zip::ZipArchive::new(Cursor::new(output)).expect("output archive");
    assert_eq!(source_archive.comment(), comment);
    assert_eq!(output_archive.comment(), comment);
}

/// Appends a paragraph without changing existing block identities or opaque parts.
#[test]
fn appending_paragraph_preserves_existing_blocks_and_package_members() {
    let original = package(
        r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>First</w:t></w:r></w:p><w:tbl><w:tr><w:tc><w:p><w:r><w:t>Cell text</w:t></w:r></w:p></w:tc></w:tr></w:tbl><w:sectPr/></w:body></w:document>"#,
    );
    let document_id = semantic_document_id(EntityId::from_bytes([9; 16]));
    let imported = import_docx(&original, document_id).expect("import original");
    let section = &imported.document.sections[0];
    let original_blocks = section.blocks.clone();
    let updated = append_simple_paragraph(
        &original,
        document_id,
        section.id,
        "Appended text & details",
    )
    .expect("append exact-section paragraph");
    let reopened = import_docx(&updated, document_id).expect("reopen appended DOCX");
    let blocks = &reopened.document.sections[0].blocks;
    assert_eq!(&blocks[..original_blocks.len()], original_blocks);
    let Some(scriberig_domain_document::DocumentBlock::Paragraph(appended)) = blocks.last() else {
        panic!("appended block is a paragraph");
    };
    assert_eq!(
        appended
            .runs
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>(),
        "Appended text & details"
    );
    let mut archive = zip::ZipArchive::new(Cursor::new(&updated)).expect("open updated package");
    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .expect("main document")
        .read_to_string(&mut xml)
        .expect("read document XML");
    assert!(xml.find("Appended text &amp; details").unwrap() < xml.find("<w:sectPr").unwrap());
    let mut opaque = Vec::new();
    archive
        .by_name("customXml/opaque.bin")
        .expect("preserved opaque member")
        .read_to_end(&mut opaque)
        .expect("read opaque member");
    assert_eq!(opaque, b"opaque payload\0");
}

/// Rejects paragraphs with multiple runs rather than changing inline styling silently.
#[test]
fn complex_paragraph_edit_is_rejected_without_guessing_style() {
    let original = package(
        r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Styled</w:t></w:r><w:r><w:rPr><w:b/></w:rPr><w:t> text</w:t></w:r></w:p></w:body></w:document>"#,
    );
    let document_id = semantic_document_id(EntityId::from_bytes([5; 16]));
    let imported = import_docx(&original, document_id).expect("import original");
    assert!(!imported.editable);
    assert!(
        imported
            .diagnostic
            .as_deref()
            .unwrap()
            .contains("no paragraph supported by the safe writer")
    );
    let scriberig_domain_document::DocumentBlock::Paragraph(paragraph) =
        &imported.document.sections[0].blocks[0]
    else {
        panic!("expected paragraph");
    };
    let error = replace_simple_paragraph(&original, document_id, paragraph.id, "Replacement")
        .expect_err("multi-run text must remain read-only");
    assert!(error.contains("exactly one text run"));
}

/// Rejects replacement boundary spaces that Word could trim during reopening.
#[test]
fn boundary_whitespace_replacement_is_rejected() {
    let original = package(
        r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Existing</w:t></w:r></w:p></w:body></w:document>"#,
    );
    let document_id = semantic_document_id(EntityId::from_bytes([6; 16]));
    let imported = import_docx(&original, document_id).expect("import original");
    let scriberig_domain_document::DocumentBlock::Paragraph(paragraph) =
        &imported.document.sections[0].blocks[0]
    else {
        panic!("expected paragraph");
    };
    let error = replace_simple_paragraph(&original, document_id, paragraph.id, " padded ")
        .expect_err("boundary whitespace must not be silently normalized");
    assert!(error.contains("boundary whitespace"));
}

/// Rejects malformed relationship XML during staged package validation.
#[test]
fn malformed_document_relationship_part_fails_package_validation() {
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    archive
        .start_file("word/document.xml", SimpleFileOptions::default())
        .expect("document entry");
    archive
        .write_all(br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body/></w:document>"#)
        .expect("document XML");
    archive
        .start_file("word/_rels/document.xml.rels", SimpleFileOptions::default())
        .expect("relationships entry");
    archive
        .write_all(b"<Relationships><Relationship")
        .expect("malformed relationship XML");
    let bytes = archive.finish().expect("finish DOCX").into_inner();

    let error = scriberig_domain_docx::validate_package(&bytes)
        .expect_err("broken relationship XML must fail staged validation");
    assert!(error.contains("relationship XML"));
}

/// Reports a structured diagnostic when a package relation targets a missing part.
#[test]
fn missing_internal_relationship_target_fails_package_validation() {
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    archive
        .start_file("word/document.xml", SimpleFileOptions::default())
        .expect("document entry");
    archive
        .write_all(br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Text</w:t></w:r></w:p></w:body></w:document>"#)
        .expect("document XML");
    archive
        .start_file("word/_rels/document.xml.rels", SimpleFileOptions::default())
        .expect("relationships entry");
    archive
        .write_all(br#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>"#)
        .expect("relationship XML");
    let bytes = archive.finish().expect("finish DOCX").into_inner();

    let error = scriberig_domain_docx::validate_package(&bytes)
        .expect_err("missing relationship target must fail validation");
    assert!(error.contains("rId1"));
    assert!(error.contains("word/styles.xml"));
    assert!(error.contains("missing package member"));
}

/// Verifies main document text and unrelated package member bytes after editing.
///
/// - `bytes`: Rewritten DOCX package.
///
/// Panics when the package cannot reopen or preserved content changed.
fn validate_members(bytes: &[u8]) {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("open rewritten package");
    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .expect("main document")
        .read_to_string(&mut xml)
        .expect("read main document");
    assert!(xml.contains("Proposed site &amp; access"));
    assert!(xml.contains("w:name=\"preserve-anchor\""));
    assert!(xml.contains("<w:bookmarkEnd w:id=\"1\"/>"));
    let mut opaque = Vec::new();
    archive
        .by_name("customXml/opaque.bin")
        .expect("preserved member")
        .read_to_end(&mut opaque)
        .expect("read opaque member");
    assert_eq!(opaque, b"opaque payload\0");
    let mut header = Vec::new();
    archive
        .by_name("word/header1.xml")
        .expect("opaque header remains")
        .read_to_end(&mut header)
        .expect("read opaque header");
    assert_eq!(header, b"<w:hdr>opaque header</w:hdr>");
    let mut footer = Vec::new();
    archive
        .by_name("word/footer1.xml")
        .expect("opaque footer remains")
        .read_to_end(&mut footer)
        .expect("read opaque footer");
    assert_eq!(footer, b"<w:ftr>opaque footer</w:ftr>");
    let mut comments = Vec::new();
    archive
        .by_name("word/comments.xml")
        .expect("opaque comments remain")
        .read_to_end(&mut comments)
        .expect("read opaque comments");
    assert_eq!(comments, b"<w:comments>opaque comment</w:comments>");
    let mut styles = Vec::new();
    archive
        .by_name("word/styles.xml")
        .expect("referenced paragraph styles remain")
        .read_to_end(&mut styles)
        .expect("read styles");
    assert_eq!(
        styles,
        b"<w:styles xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:style w:styleId=\"BodyText\"><w:name w:val=\"Body Text\"/></w:style></w:styles>"
    );
    assert!(xml.contains("<w:pStyle w:val=\"BodyText\"/>"));
}
