//! Imports PDF structure into the universal semantic model while preserving provenance.
#![allow(missing_docs)]

mod error;
mod extract;
mod geometry;
mod import;
mod provenance;
mod schema;
pub use error::PdfError;
pub use extract::extract;
pub use geometry::{PageGeometry, Rectangle};
pub use import::{PdfSourceArtifact, import_source};
pub use provenance::PdfProvenance;
pub use schema::PdfEntityType;
use scriberig_core::EntityId;

/// An imported PDF document with exact source bytes and derived structure.
#[derive(Clone, Debug, PartialEq)]
pub struct PdfDocument {
    pub source: PdfSourceArtifact,
    pub pages: Vec<PdfPage>,
    pub regions: Vec<TextRegion>,
}
impl PdfDocument {
    /// Imports exact PDF bytes as a document.
    ///
    /// # Parameters
    /// - `source`: Exact imported source artifact.
    ///
    /// # Returns
    /// An empty document ready for derived extraction.
    #[must_use]
    pub fn from_source(source: PdfSourceArtifact) -> Self {
        Self {
            source,
            pages: Vec::new(),
            regions: Vec::new(),
        }
    }
    /// Derives a stable child identifier without using parser-private offsets.
    ///
    /// # Parameters
    /// - `crate`: Document providing the source digest.
    /// - `kind`: Child semantic kind.
    /// - `index`: Stable ordinal within that kind.
    ///
    /// # Returns
    /// A deterministic semantic entity identifier.
    pub(crate) fn child_id(&self, kind: &str, index: u32) -> EntityId {
        let mut h = blake3::Hasher::new();
        h.update(&self.source.digest);
        h.update(kind.as_bytes());
        h.update(&index.to_be_bytes());
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&h.finalize().as_bytes()[..16]);
        EntityId::new(bytes)
    }
}
/// One ordered PDF page and its derived child handles.
#[derive(Clone, Debug, PartialEq)]
pub struct PdfPage {
    pub id: EntityId,
    pub number: u32,
    pub geometry: PageGeometry,
    pub regions: Vec<EntityId>,
}
/// One derived text region with source provenance.
#[derive(Clone, Debug, PartialEq)]
pub struct TextRegion {
    pub id: EntityId,
    pub page_number: u32,
    pub text: String,
    pub provenance: PdfProvenance,
}
#[cfg(test)]
mod tests {
    use super::*;
    /// Preserves source bytes while deriving page structure.
    #[test]
    fn import_preserves_source_and_derives_pages() {
        let source = import_source(b"%PDF-1.7 /Type /Page (hello)".to_vec()).unwrap();
        let doc = extract(&PdfDocument::from_source(source.clone()), "baseline", "v1");
        assert_eq!(doc.source.bytes, source.bytes);
        assert_eq!(doc.pages.len(), 1);
        assert_eq!(doc.regions[0].provenance.source_digest, source.digest);
        assert!(doc.regions[0].provenance.region.is_some());
    }
}
