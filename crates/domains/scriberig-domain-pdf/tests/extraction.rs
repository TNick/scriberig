//! Verifies derived PDF hierarchy and producer provenance.
use scriberig_domain_pdf::{PdfDocument, extract, import_source};

/// Keeps producer profiles visible and semantic IDs independent of offsets.
#[test]
fn extraction_records_page_provenance() {
    let source = import_source(b"%PDF-1.7 /Type /Page (hello)".to_vec()).expect("valid PDF");
    let document = PdfDocument::from_source(source);
    let first = extract(&document, "parser", "v1");
    let second = extract(&document, "parser", "v2");
    assert_eq!(first.pages[0].number, 1);
    assert_eq!(first.regions[0].provenance.producer, "parser");
    assert_ne!(
        first.regions[0].provenance.profile,
        second.regions[0].provenance.profile
    );
    assert_eq!(first.regions[0].provenance.confidence, None);
}
