//! Checks staged XLSX packages preserve non-cell content and exact member digests.

use std::collections::BTreeMap;
use std::io::{Cursor, Read};

use crate::XlsxAdapterError;

/// Requires every non-worksheet OOXML part to remain exact after decompression.
///
/// # Parameters
/// - `old`: Exact base package bytes.
/// - `new`: Candidate package bytes.
///
/// # Returns
/// Unit when member sets match and opaque parts retain their content digests.
///
/// # Errors
/// Returns an error when a part is added, removed, unreadable, or unexpectedly changed.
pub fn validate_opaque_members(old: &[u8], new: &[u8]) -> Result<(), XlsxAdapterError> {
    let old_parts = member_digests(old)?;
    let new_parts = member_digests(new)?;
    if old_parts.keys().ne(new_parts.keys()) {
        return Err(validation_error(
            "candidate added or removed an XLSX package member",
        ));
    }
    for (name, old_digest) in old_parts {
        if !is_cell_storage_part(&name) && new_parts.get(&name) != Some(&old_digest) {
            return Err(validation_error(&format!(
                "candidate changed opaque or unrelated package part {name}"
            )));
        }
    }
    Ok(())
}

/// Determines whether a package member may change for a scalar cell operation.
///
/// # Parameters
/// - `name`: Exact normalized ZIP member name.
///
/// # Returns
/// True only for worksheet XML or shared-string storage.
fn is_cell_storage_part(name: &str) -> bool {
    (name.starts_with("xl/worksheets/")
        && std::path::Path::new(name)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("xml"))
        && !name.contains("/_rels/"))
        || name == "xl/sharedStrings.xml"
}

/// Computes content digests for every readable ZIP member.
///
/// # Parameters
/// - `bytes`: Exact ZIP package bytes.
///
/// # Returns
/// Member names mapped to BLAKE3 content digests.
///
/// # Errors
/// Returns an error for malformed ZIP directories or unreadable member data.
pub fn member_digests(bytes: &[u8]) -> Result<BTreeMap<String, [u8; 32]>, XlsxAdapterError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| validation_error(&format!("invalid XLSX ZIP directory: {error}")))?;
    let mut members = BTreeMap::new();
    for index in 0..archive.len() {
        let mut member = archive
            .by_index(index)
            .map_err(|error| validation_error(&format!("invalid XLSX ZIP member: {error}")))?;
        let name = member.name().to_owned();
        let mut hasher = blake3::Hasher::new();
        let mut buffer = [0_u8; 16 * 1024];
        loop {
            let count = member.read(&mut buffer).map_err(|error| {
                validation_error(&format!("XLSX member {name} failed read/CRC: {error}"))
            })?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
        }
        if members
            .insert(name, *hasher.finalize().as_bytes())
            .is_some()
        {
            return Err(validation_error(
                "candidate package contains duplicate member names",
            ));
        }
    }
    Ok(members)
}

/// Creates one structured XLSX validation failure.
///
/// # Parameters
/// - `message`: Safe reason validation failed.
///
/// # Returns
/// Adapter validation error without source bytes.
pub fn validation_error(message: &str) -> XlsxAdapterError {
    XlsxAdapterError {
        code: "xlsx_candidate_invalid",
        message: message.to_owned(),
    }
}
