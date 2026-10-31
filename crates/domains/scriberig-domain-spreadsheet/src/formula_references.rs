//! Extracts inspectable A1 cell and range references from authored formulas.

use crate::Formula;

/// Identifies one A1 cell coordinate and its absolute-reference markers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellAddress {
    /// One-based row number.
    pub row: u32,
    /// One-based column number.
    pub column: u32,
    /// Whether the authored row uses an absolute `$` marker.
    pub absolute_row: bool,
    /// Whether the authored column uses an absolute `$` marker.
    pub absolute_column: bool,
}

/// Represents one recognized cell or rectangular range reference in a formula.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormulaReference {
    /// Authored worksheet qualifier, absent for a same-sheet reference.
    pub sheet: Option<String>,
    /// First cell in the reference or rectangular range.
    pub start: CellAddress,
    /// Last cell for a rectangular range, absent for a single cell.
    pub end: Option<CellAddress>,
}

impl Formula {
    /// Extracts conventional A1 references without evaluating the formula.
    ///
    /// This inspects conventional A1 cell and rectangular references with
    /// optional absolute markers and quoted or unquoted sheet names. It skips
    /// double-quoted string literals. Named ranges, structured references,
    /// external-workbook syntax, and formula evaluation are outside this
    /// reader's contract. The authored expression and cached value are untouched.
    ///
    /// # Returns
    /// Recognized A1 cell and range references in source order.
    #[must_use]
    pub fn a1_references(&self) -> Vec<FormulaReference> {
        extract_references(&self.expression)
    }
}

/// Scans formula bytes for bounded A1 references outside string literals.
///
/// # Parameters
/// - `expression`: Authored formula expression.
///
/// # Returns
/// Recognized references in source order.
fn extract_references(expression: &str) -> Vec<FormulaReference> {
    let bytes = expression.as_bytes();
    let mut references = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'"' {
            index = skip_string(bytes, index + 1);
            continue;
        }
        if !is_token_start(bytes, index) {
            index += 1;
            continue;
        }
        if let Some((reference, end)) = parse_reference(bytes, index) {
            references.push(reference);
            index = end;
        } else {
            index += 1;
        }
    }
    references
}

/// Skips a double-quoted formula string, including doubled quote escapes.
///
/// # Parameters
/// - `bytes`: UTF-8 expression bytes.
/// - `index`: First byte after the opening quote.
///
/// # Returns
/// Byte position after the closing quote or the expression end.
fn skip_string(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() {
        if bytes[index] == b'"' {
            if bytes.get(index + 1) == Some(&b'"') {
                index += 2;
                continue;
            }
            return index + 1;
        }
        index += 1;
    }
    index
}

/// Determines whether a token may begin at one byte offset.
///
/// # Parameters
/// - `bytes`: Formula expression bytes.
/// - `index`: Candidate byte offset.
///
/// # Returns
/// True when the preceding byte cannot continue an identifier or cell token.
fn is_token_start(bytes: &[u8], index: usize) -> bool {
    index == 0 || !is_identifier_byte(bytes[index - 1])
}

/// Parses an optional sheet prefix, cell address, and optional range endpoint.
///
/// # Parameters
/// - `bytes`: Formula expression bytes.
/// - `start`: Candidate token offset.
///
/// # Returns
/// A recognized reference and its ending byte offset.
fn parse_reference(bytes: &[u8], start: usize) -> Option<(FormulaReference, usize)> {
    let (sheet, address_start) =
        parse_sheet_prefix(bytes, start).map_or((None, start), |(sheet, next)| (Some(sheet), next));
    let (cell_start, mut next) = parse_address(bytes, address_start)?;
    let end = if bytes.get(next) == Some(&b':') {
        let (range_end, range_next) = parse_address(bytes, next + 1)?;
        next = range_next;
        Some(range_end)
    } else {
        None
    };
    if bytes
        .get(next)
        .is_some_and(|byte| is_identifier_byte(*byte))
    {
        return None;
    }
    Some((
        FormulaReference {
            sheet,
            start: cell_start,
            end,
        },
        next,
    ))
}

/// Parses a quoted or unquoted worksheet qualifier followed by `!`.
///
/// # Parameters
/// - `bytes`: Formula expression bytes.
/// - `start`: Candidate qualifier offset.
///
/// # Returns
/// Authored sheet name and address start, or `None` when no qualifier is present.
fn parse_sheet_prefix(bytes: &[u8], start: usize) -> Option<(String, usize)> {
    if bytes.get(start) == Some(&b'\'') {
        let mut name = Vec::new();
        let mut index = start + 1;
        while index < bytes.len() {
            match bytes[index] {
                b'\'' if bytes.get(index + 1) == Some(&b'\'') => {
                    name.push(b'\'');
                    index += 2;
                }
                b'\'' if bytes.get(index + 1) == Some(&b'!') => {
                    return Some((String::from_utf8(name).ok()?, index + 2));
                }
                b'\'' => return None,
                byte => {
                    name.push(byte);
                    index += 1;
                }
            }
        }
        return None;
    }
    let mut index = start;
    while bytes
        .get(index)
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.'))
    {
        index += 1;
    }
    if index > start && bytes.get(index) == Some(&b'!') {
        Some((
            std::str::from_utf8(&bytes[start..index]).ok()?.to_owned(),
            index + 1,
        ))
    } else {
        None
    }
}

/// Parses one bounded A1 cell address.
///
/// # Parameters
/// - `bytes`: Formula expression bytes.
/// - `start`: Address byte offset.
///
/// # Returns
/// Cell address and the next byte offset, or `None` for invalid/out-of-grid syntax.
fn parse_address(bytes: &[u8], start: usize) -> Option<(CellAddress, usize)> {
    let mut index = start;
    let absolute_column = bytes.get(index) == Some(&b'$');
    if absolute_column {
        index += 1;
    }
    let column_start = index;
    let mut column = 0_u32;
    while bytes.get(index).is_some_and(u8::is_ascii_alphabetic) {
        column = column
            .checked_mul(26)?
            .checked_add(u32::from(bytes[index].to_ascii_uppercase() - b'A' + 1))?;
        index += 1;
    }
    if index == column_start || column > 16_384 {
        return None;
    }
    let absolute_row = bytes.get(index) == Some(&b'$');
    if absolute_row {
        index += 1;
    }
    let row_start = index;
    let mut row = 0_u32;
    while bytes.get(index).is_some_and(u8::is_ascii_digit) {
        row = row
            .checked_mul(10)?
            .checked_add(u32::from(bytes[index] - b'0'))?;
        index += 1;
    }
    if index == row_start || row == 0 || row > 1_048_576 {
        return None;
    }
    Some((
        CellAddress {
            row,
            column,
            absolute_row,
            absolute_column,
        },
        index,
    ))
}

/// Identifies bytes that continue an A1 token or sheet identifier.
///
/// # Parameters
/// - `byte`: Candidate ASCII byte.
///
/// # Returns
/// True for ASCII letters, digits, underscore, and period.
fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.')
}

#[cfg(test)]
mod tests {
    use crate::{CellValue, Formula};

    /// Verifies A1 cells, ranges, absolute markers, sheets, and string skipping.
    #[test]
    fn extracts_authored_a1_references_without_evaluating_cached_values() {
        let formula = Formula {
            expression: "=SUM($A$3:B4)+ 'Budget Plan'!C5 + Sheet2!$D6 + \"A1\"".to_owned(),
            cached_value: Some(Box::new(CellValue::Number("999".to_owned()))),
        };
        let references = formula.a1_references();
        assert_eq!(references.len(), 3);
        assert_eq!(references[0].start.row, 3);
        assert_eq!(references[0].start.column, 1);
        assert!(references[0].start.absolute_row);
        assert!(references[0].start.absolute_column);
        assert_eq!(
            references[0].end.as_ref().map(|address| address.row),
            Some(4)
        );
        assert_eq!(references[1].sheet.as_deref(), Some("Budget Plan"));
        assert_eq!(references[1].start.column, 3);
        assert_eq!(references[2].sheet.as_deref(), Some("Sheet2"));
        assert!(references[2].start.absolute_column);
        assert_eq!(references[2].start.row, 6);
        assert_eq!(
            formula.cached_value,
            Some(Box::new(CellValue::Number("999".to_owned())))
        );
    }

    /// Verifies out-of-grid identifiers are not reported as cell references.
    #[test]
    fn rejects_a1_shaped_values_outside_spreadsheet_grid() {
        let formula = Formula {
            expression: "=XFE1+A1048577+TaxRate".to_owned(),
            cached_value: None,
        };
        assert_eq!(formula.a1_references().len(), 0);
    }
}
