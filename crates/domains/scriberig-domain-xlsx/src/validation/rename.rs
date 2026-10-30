//! Verifies sheet renames against the retained-package reference updater.

use crate::{XlsxAdapterError, XlsxImport, rename_sheet};

/// Recognizes and verifies one isolated worksheet rename.
///
/// # Parameters
/// - `old`: Exact base package bytes.
/// - `new`: Candidate package bytes.
/// - `original`: Imported base workbook.
/// - `candidate`: Imported candidate workbook.
///
/// # Returns
/// `Some(())` when the candidate exactly represents one supported sheet rename;
/// `None` when it is not a single-rename candidate.
///
/// # Errors
/// Returns an error when a detected rename contains extra, missing, or altered parts.
pub(super) fn validate_candidate(
    old: &[u8],
    new: &[u8],
    original: &XlsxImport,
    candidate: &XlsxImport,
) -> Result<Option<()>, XlsxAdapterError> {
    if original.worksheets.len() != candidate.worksheets.len()
        || original
            .worksheets
            .iter()
            .zip(&candidate.worksheets)
            .any(|(before, after)| before.id != after.id)
    {
        return Ok(None);
    }
    let changed = original
        .worksheets
        .iter()
        .zip(&candidate.worksheets)
        .filter(|(before, after)| before.name != after.name)
        .collect::<Vec<_>>();
    if changed.len() != 1 {
        return Ok(None);
    }
    let (before, after) = changed[0];
    let expected = rename_sheet(old, &before.name, &after.name)?;
    if super::member_digests(&expected)? != super::member_digests(new)? {
        return Err(super::validation_error(
            "sheet rename changed unsupported or unrelated workbook content",
        ));
    }
    Ok(Some(()))
}
