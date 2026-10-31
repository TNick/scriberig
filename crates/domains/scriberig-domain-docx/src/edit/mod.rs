//! Replaces structurally simple DOCX paragraphs while preserving other package parts.

use scriberig_core::EntityId;

mod append;
mod identity;
mod probe;
mod rewrite;

pub use append::append_simple_paragraph;
pub(crate) use probe::probe_safe_paragraph_edit;
pub use rewrite::replace_simple_paragraph;

pub(super) const MAIN_DOCUMENT: &str = "word/document.xml";
pub(super) const WORD_NAMESPACE: &str =
    "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
pub(super) const MAX_PACKAGE_BYTES: u64 = 256 * 1024 * 1024;

use crate::import_docx;

/// Validates that a DOCX main document and optional hyperlink relations reopen.
///
/// # Parameters
/// - `bytes`: Candidate package bytes to reopen.
///
/// # Returns
/// Unit when the package and semantic main document reopen successfully.
///
/// # Errors
/// Returns an error when the package, main document, or XML is invalid.
pub fn validate_package(bytes: &[u8]) -> Result<(), String> {
    import_docx(bytes, EntityId::from_bytes([0; 16]))
        .map_err(|error| format!("DOCX semantic package reopen failed: {error}"))?;
    crate::relationships::validate_relationship_targets(bytes)?;
    Ok(())
}
