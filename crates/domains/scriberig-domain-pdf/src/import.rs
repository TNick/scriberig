//! Defines exact PDF source artifact preservation and document import.

use crate::error::PdfError;

/// Preserves the exact imported PDF bytes and their content digest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfSourceArtifact {
    pub bytes: Vec<u8>,
    pub digest: [u8; 32],
}

/// Imports bytes as an exact PDF source artifact.
///
/// # Parameters
/// - `bytes`: Exact PDF bytes to preserve.
///
/// # Returns
/// The immutable source artifact.
///
/// # Errors
/// Returns an error for empty or non-PDF input.
pub fn import_source(bytes: impl Into<Vec<u8>>) -> Result<PdfSourceArtifact, PdfError> {
    let bytes = bytes.into();
    if bytes.is_empty() {
        return Err(PdfError::EmptySource);
    }
    if !bytes.starts_with(b"%PDF-") {
        return Err(PdfError::InvalidHeader);
    }
    let digest = blake3::hash(&bytes).as_bytes().to_owned();
    Ok(PdfSourceArtifact { bytes, digest })
}
