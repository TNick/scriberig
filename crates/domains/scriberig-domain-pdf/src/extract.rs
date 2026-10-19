//! Defines deterministic baseline text extraction and page hierarchy.

use crate::{
    PdfDocument, PdfPage, TextRegion,
    geometry::{PageGeometry, Rectangle},
    provenance::PdfProvenance,
};

/// Extracts pages and literal parenthesized text regions from a source artifact.
///
/// The baseline extractor intentionally retains derived output alongside the exact source.
///
/// - `document`: Imported PDF document to enrich.
/// - `producer`: Extractor identity.
/// - `profile`: Extractor profile/version.
///
/// A document with ordered pages and derived text regions.
/// # Returns
/// The operation result.
pub fn extract(
    document: &PdfDocument,
    producer: impl Into<String>,
    profile: impl Into<String>,
) -> PdfDocument {
    let producer = producer.into();
    let profile = profile.into();
    let page_count = document
        .source
        .bytes
        .windows(6)
        .filter(|w| *w == b"/Type ")
        .count()
        .max(1)
        .try_into()
        .unwrap_or(u32::MAX);
    let mut pages = Vec::new();
    let mut regions = Vec::new();
    let text = String::from_utf8_lossy(&document.source.bytes);
    let mut n = 0u32;
    for part in text.split('(').skip(1) {
        if let Some(value) = part.split(')').next() {
            if !value.is_empty() && value.len() < 4096 {
                n += 1;
                regions.push(TextRegion {
                    id: document.child_id("region", n),
                    page_number: 1,
                    text: value.to_owned(),
                    provenance: PdfProvenance {
                        source_digest: document.source.digest,
                        page_number: 1,
                        producer: producer.clone(),
                        profile: profile.clone(),
                        region: Some(Rectangle {
                            x: 0.0,
                            y: 0.0,
                            width: f32::from(
                                u16::try_from(value.len().min(65_535)).unwrap_or(u16::MAX),
                            ),
                            height: 12.0,
                        }),
                        confidence: None,
                    },
                });
            }
        }
    }
    for page_number in 1..=page_count {
        pages.push(PdfPage {
            id: document.child_id("page", page_number),
            number: page_number,
            geometry: PageGeometry {
                width: 595.0,
                height: 842.0,
            },
            regions: regions
                .iter()
                .filter(|r| r.page_number == page_number)
                .map(|r| r.id)
                .collect(),
        });
    }
    PdfDocument {
        source: document.source.clone(),
        pages,
        regions,
    }
}
