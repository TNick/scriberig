//! Applies narrow DXF edits without reserializing unsupported source records.

use super::{DxfEditError, edit_error};

mod parse;
use parse::{Pair, find_target, pair_value, parse_pairs};

/// Replaces one target entity's existing group-code values in exact source bytes.
///
/// Only a unique target record and unique requested value codes are accepted.
/// Every byte outside the selected value lines remains unchanged.
///
/// # Parameters
/// - `bytes`: Exact text DXF bytes.
/// - `entity_type`: Target DXF record type such as `LINE` or `TEXT`.
/// - `handle`: Native handle identifying the target entity.
/// - `replacements`: Existing group-code values to replace.
///
/// # Returns
/// Updated bytes with every unselected source byte retained exactly.
///
/// # Errors
/// Returns a structured edit error when pairs, section, target, or replacement
/// fields are malformed, absent, or ambiguous.
pub(super) fn replace_record_values(
    bytes: &[u8],
    entity_type: &str,
    handle: &str,
    replacements: &[(i16, Vec<u8>)],
) -> Result<Vec<u8>, DxfEditError> {
    let pairs = parse_pairs(bytes)?;
    let (start, end) = find_target(&pairs, bytes, entity_type, handle)?;
    let mut edits = Vec::new();
    for (code, value) in replacements {
        let matching = pairs[start..end]
            .iter()
            .filter(|pair| pair.code == *code)
            .collect::<Vec<_>>();
        if matching.len() != 1 {
            return Err(edit_error(
                "dxf_edit_opaque_ambiguous",
                "target record does not contain one unambiguous value for each requested field",
            ));
        }
        let pair = matching[0];
        if value.contains(&b'\n') || value.contains(&b'\r') {
            return Err(edit_error(
                "dxf_edit_opaque_encoding_unsupported",
                "replacement value cannot contain a record separator",
            ));
        }
        edits.push((pair.value_start, pair.value_end, value.as_slice()));
    }
    edits.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));
    let mut output = bytes.to_vec();
    for (start, end, value) in edits {
        output.splice(start..end, value.iter().copied());
    }
    Ok(output)
}

/// Reports whether a parsed text DXF can support exact raw-record preservation.
///
/// # Parameters
/// - `bytes`: Exact source bytes to inspect.
///
/// # Returns
/// Whether the source has a valid text code-pair structure and ENTITIES section.
#[must_use]
pub fn supports_opaque_preserving_edits(bytes: &[u8]) -> bool {
    let Ok(pairs) = parse_pairs(bytes) else {
        return false;
    };
    let mut in_entities = false;
    for (index, pair) in pairs.iter().enumerate() {
        if pair.code != 0 {
            continue;
        }
        let value = pair_value(pair, bytes);
        if value.eq_ignore_ascii_case(b"SECTION")
            && pairs.get(index + 1).is_some_and(|candidate| {
                candidate.code == 2
                    && pair_value(candidate, bytes).eq_ignore_ascii_case(b"ENTITIES")
            })
        {
            in_entities = true;
            continue;
        }
        if !in_entities {
            continue;
        }
        if value.eq_ignore_ascii_case(b"ENDSEC") {
            in_entities = false;
            continue;
        }
        if ![
            b"LINE".as_slice(),
            b"TEXT",
            b"POLYLINE",
            b"LWPOLYLINE",
            b"CIRCLE",
            b"ARC",
        ]
        .iter()
        .any(|kind| value.eq_ignore_ascii_case(kind))
        {
            continue;
        }
        let end = pairs[index + 1..]
            .iter()
            .position(|candidate| candidate.code == 0)
            .map_or(pairs.len(), |next| index + 1 + next);
        if pairs[index + 1..end]
            .iter()
            .filter(|candidate| candidate.code == 5)
            .count()
            == 1
        {
            return true;
        }
    }
    false
}

/// Proves a candidate changes only allowed value fields on one exact entity.
///
/// The original and candidate must retain identical pair counts, group-code
/// ordering, target bounds, group-code lines, and line delimiters. Only values
/// selected by `allowed_codes` inside the target record may differ.
///
/// # Parameters
/// - `baseline`: Exact source representation before the edit.
/// - `candidate`: Exact staged representation after the edit.
/// - `handle`: Native handle identifying the target.
/// - `allowed_codes`: Group-code values the semantic operation may change.
///
/// # Returns
/// Unit when every unapproved source byte and opaque record remains unchanged.
///
/// # Errors
/// Returns a structured validation error when source structure or preservation
/// cannot be proven.
pub fn verify_opaque_preservation(
    baseline: &[u8],
    candidate: &[u8],
    handle: &str,
    allowed_codes: &[i16],
) -> Result<(), DxfEditError> {
    let original = parse_pairs(baseline)?;
    let updated = parse_pairs(candidate)?;
    if original.len() != updated.len() {
        return Err(preservation_error());
    }
    let entity_type = record_type(&original, baseline, handle)?;
    let original_target = find_target(&original, baseline, &entity_type, handle)?;
    let updated_target = find_target(&updated, candidate, &entity_type, handle)?;
    for code in allowed_codes {
        if original[original_target.0..original_target.1]
            .iter()
            .filter(|pair| pair.code == *code)
            .count()
            != 1
        {
            return Err(preservation_error());
        }
    }
    if original_target != updated_target {
        return Err(preservation_error());
    }
    let mut changed = false;
    for (index, (before, after)) in original.iter().zip(&updated).enumerate() {
        if before.code != after.code
            || baseline[before.code_start..before.code_line_end]
                != candidate[after.code_start..after.code_line_end]
            || baseline[before.value_end..before.end] != candidate[after.value_end..after.end]
        {
            return Err(preservation_error());
        }
        let permitted = (original_target.0..original_target.1).contains(&index)
            && allowed_codes.contains(&before.code);
        if !permitted
            && baseline[before.value_start..before.value_end]
                != candidate[after.value_start..after.value_end]
        {
            return Err(preservation_error());
        }
        changed |= permitted
            && baseline[before.value_start..before.value_end]
                != candidate[after.value_start..after.value_end];
    }
    if changed {
        Ok(())
    } else {
        Err(preservation_error())
    }
}

/// Resolves a unique ENTITIES record type from its native handle.
///
/// # Parameters
/// - `pairs`: Parsed DXF code pairs.
/// - `bytes`: Exact source bytes for record values.
/// - `handle`: Native entity handle.
///
/// # Returns
/// Native record type such as `LINE` or `TEXT`.
///
/// # Errors
/// Returns an error when the handle has no unique record in ENTITIES.
fn record_type(pairs: &[Pair], bytes: &[u8], handle: &str) -> Result<String, DxfEditError> {
    let mut in_entities = false;
    let mut section_start = None;
    let mut records = Vec::new();
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
        if !in_entities || value.eq_ignore_ascii_case(b"SECTION") {
            continue;
        }
        let end = pairs[index + 1..]
            .iter()
            .position(|candidate| candidate.code == 0)
            .map_or(pairs.len(), |next| index + 1 + next);
        if pairs[index + 1..end].iter().any(|candidate| {
            candidate.code == 5
                && pair_value(candidate, bytes).eq_ignore_ascii_case(handle.as_bytes())
        }) {
            records.push(String::from_utf8_lossy(value).into_owned());
        }
    }
    match records.as_slice() {
        [record] => Ok(record.clone()),
        _ => Err(preservation_error()),
    }
}

/// Creates the closed error used when exact opaque preservation is unproven.
///
/// # Returns
/// A structured DXF validation failure.
fn preservation_error() -> DxfEditError {
    edit_error(
        "dxf_opaque_preservation_failed",
        "staged DXF changed source data outside the selected semantic fields",
    )
}
