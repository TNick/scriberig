//! Verifies exact PDF source import behavior.
use scriberig_domain_pdf::{PdfError, import_source};

/// Rejects non-PDF input while preserving exact accepted bytes.
#[test]
fn source_validation_is_explicit() {
    assert_eq!(import_source(Vec::<u8>::new()), Err(PdfError::EmptySource));
    let source = import_source(b"%PDF-1.7 exact".to_vec()).expect("valid PDF header");
    assert_eq!(source.bytes, b"%PDF-1.7 exact");
}
