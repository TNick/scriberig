//! Defines provenance attached to PDF-derived entities.

/// Identifies the producer and source location of a derived PDF entity.
#[derive(Clone, Debug, PartialEq)]
pub struct PdfProvenance {
    pub source_digest: [u8; 32],
    pub page_number: u32,
    pub producer: String,
    pub profile: String,
    pub region: Option<crate::geometry::Rectangle>,
    pub confidence: Option<u8>,
}
