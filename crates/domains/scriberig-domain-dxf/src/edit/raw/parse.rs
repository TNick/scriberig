//! Parses and locates records in exact text DXF source bytes.

use super::super::{DxfEditError, edit_error};

/// Stores one parsed DXF code/value pair and its exact source byte offsets.
#[derive(Clone, Debug)]
pub(super) struct Pair {
    pub(super) code: i16,
    pub(super) code_start: usize,
    pub(super) code_line_end: usize,
    pub(super) value_start: usize,
    pub(super) value_end: usize,
    pub(super) end: usize,
}

/// Checks that a byte sequence has a complete group-code/value pair structure.
///
/// # Parameters
/// - `bytes`: Exact text DXF bytes to inspect.
///
/// # Returns
/// Parsed group-code pairs with byte offsets into the original input.
///
/// # Errors
/// Returns a structured parse error for non-ASCII codes or incomplete pairs.
pub(super) fn parse_pairs(bytes: &[u8]) -> Result<Vec<Pair>, DxfEditError> {
    let mut lines = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        let end = bytes[offset..]
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(bytes.len(), |length| offset + length + 1);
        let content_end = if end > offset && bytes[end - 1] == b'\n' {
            if end > offset + 1 && bytes[end - 2] == b'\r' {
                end - 2
            } else {
                end - 1
            }
        } else {
            end
        };
        lines.push((offset, content_end, end));
        offset = end;
    }
    if lines.len() % 2 != 0 {
        return Err(edit_error(
            "dxf_edit_opaque_malformed",
            "DXF text source ends with an incomplete group-code pair",
        ));
    }
    lines
        .chunks_exact(2)
        .map(|pair| {
            let (code_start, code_end, _) = pair[0];
            let code_bytes = trim_ascii(&bytes[code_start..code_end]);
            let code_text = std::str::from_utf8(code_bytes).map_err(|_| {
                edit_error(
                    "dxf_edit_opaque_encoding_unsupported",
                    "DXF group codes are not ASCII text",
                )
            })?;
            let code = code_text.parse::<i16>().map_err(|_| {
                edit_error(
                    "dxf_edit_opaque_malformed",
                    "DXF group code is not a valid integer",
                )
            })?;
            Ok(Pair {
                code,
                code_start,
                code_line_end: pair[0].2,
                value_start: pair[1].0,
                value_end: pair[1].1,
                end: pair[1].2,
            })
        })
        .collect()
}

/// Locates one uniquely identified entity record in the ENTITIES section.
///
/// # Parameters
/// - `pairs`: Parsed source code pairs.
/// - `bytes`: Original bytes used to inspect textual marker values.
/// - `entity_type`: Requested record type.
/// - `handle`: Requested native entity handle.
///
/// # Returns
/// Start-inclusive and end-exclusive pair indexes for the target record.
///
/// # Errors
/// Returns a structured error when the section or target is missing or ambiguous.
pub(super) fn find_target(
    pairs: &[Pair],
    bytes: &[u8],
    entity_type: &str,
    handle: &str,
) -> Result<(usize, usize), DxfEditError> {
    let mut in_entities = false;
    let mut section_start = None;
    let mut targets = Vec::new();
    for (index, pair) in pairs.iter().enumerate() {
        if pair.code != 0 {
            continue;
        }
        let value = pair_value(pair, bytes);
        if value.eq_ignore_ascii_case(b"SECTION") {
            section_start = Some(index);
        } else if value.eq_ignore_ascii_case(b"ENDSEC") {
            in_entities = false;
            section_start = None;
        }
        if let Some(section) = section_start.take()
            && pairs.get(section + 1).is_some_and(|candidate| {
                candidate.code == 2
                    && pair_value(candidate, bytes).eq_ignore_ascii_case(b"ENTITIES")
            })
        {
            in_entities = true;
        }
        if !in_entities || !value.eq_ignore_ascii_case(entity_type.as_bytes()) {
            continue;
        }
        let end = pairs[index + 1..]
            .iter()
            .position(|candidate| candidate.code == 0)
            .map_or(pairs.len(), |next| index + 1 + next);
        let handles = pairs[index + 1..end]
            .iter()
            .filter(|candidate| candidate.code == 5)
            .map(|candidate| pair_value(candidate, bytes))
            .collect::<Vec<_>>();
        if handles.len() == 1 && handles[0].eq_ignore_ascii_case(handle.as_bytes()) {
            targets.push((index, end));
        }
    }
    match targets.as_slice() {
        [target] => Ok(*target),
        [] => Err(edit_error(
            "dxf_target_not_found",
            "native handle does not identify the requested DXF record",
        )),
        _ => Err(edit_error(
            "dxf_edit_opaque_ambiguous",
            "native handle identifies more than one DXF record",
        )),
    }
}

/// Returns one group-pair value without interpreting its source encoding.
///
/// # Parameters
/// - `pair`: Parsed value offsets.
/// - `bytes`: Original DXF byte sequence.
///
/// # Returns
/// Pair value with surrounding ASCII whitespace removed.
pub(super) fn pair_value<'a>(pair: &Pair, bytes: &'a [u8]) -> &'a [u8] {
    trim_ascii(&bytes[pair.value_start..pair.value_end])
}

/// Trims ASCII whitespace from one byte slice.
///
/// # Parameters
/// - `bytes`: Input value line.
///
/// # Returns
/// The input slice without surrounding ASCII whitespace.
fn trim_ascii(mut bytes: &[u8]) -> &[u8] {
    while bytes.first().is_some_and(u8::is_ascii_whitespace) {
        bytes = &bytes[1..];
    }
    while bytes.last().is_some_and(u8::is_ascii_whitespace) {
        bytes = &bytes[..bytes.len() - 1];
    }
    bytes
}
