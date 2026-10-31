//! Parses bounded worksheet range arguments for the TUI spreadsheet command.

/// Parses an optional A1 range and zero-based page offset.
///
/// # Parameters
/// - `arguments`: Command arguments after `/spreadsheet`.
///
/// # Returns
/// Inclusive coordinate bounds and page offset.
///
/// # Errors
/// Returns a concise usage or coordinate validation message.
pub(super) fn parse(
    arguments: &str,
) -> Result<(Option<scriberig_domain_spreadsheet::CellRange>, usize), &'static str> {
    let mut tokens = arguments.split_whitespace();
    let first = tokens.next();
    let (range, offset_token) = match first {
        Some(value) if value.contains(':') => (Some(parse_range(value)?), tokens.next()),
        Some(value) => (None, Some(value)),
        None => (None, None),
    };
    let offset = offset_token.map_or(Ok(0), |value| {
        value
            .parse::<usize>()
            .map_err(|_| "Page offset must be a non-negative integer.")
    })?;
    if tokens.next().is_some() {
        return Err("Usage: /spreadsheet [A1:C20] [offset].");
    }
    Ok((range, offset))
}

/// Resolves the exact project, worksheet, and snapshot used by spreadsheet commands.
///
/// # Parameters
/// - `state`: Current semantic selection and retained worksheet context.
///
/// # Returns
/// Exact project, snapshot, and worksheet identities.
///
/// # Errors
/// Returns a safe message when a project or exact semantic context is unavailable.
pub(super) fn scope(
    state: &crate::state::TuiState,
) -> Result<
    (
        scriberig_core::ProjectId,
        scriberig_core::SnapshotId,
        scriberig_core::EntityId,
    ),
    &'static str,
> {
    let Some(agent_selection) = state.agent_selection.as_ref() else {
        return Err("Select a worksheet at an exact snapshot first.");
    };
    let Some(project) = agent_selection.project else {
        return Err("Select a project before inspecting spreadsheet cells.");
    };
    let snapshot = state
        .selection
        .as_ref()
        .and_then(|selection| selection.snapshot)
        .unwrap_or(agent_selection.snapshot);
    let worksheet = state.spreadsheet_worksheet.unwrap_or_else(|| {
        state
            .selection
            .as_ref()
            .map_or(agent_selection.entity, |selection| selection.entity)
    });
    Ok((project, snapshot, worksheet))
}

/// Parses a coordinate and its typed value for a candidate edit command.
///
/// # Parameters
/// - `arguments`: Text after the set-value or set-formula command.
/// - `formula_only`: Whether the command accepts formula expressions only.
///
/// # Returns
/// One-based target coordinates and authored domain value.
///
/// # Errors
/// Returns a concise coordinate, type, or value usage message.
pub(super) fn parse_edit(
    arguments: &str,
    formula_only: bool,
) -> Result<(u32, u32, scriberig_domain_spreadsheet::CellValue), &'static str> {
    let (address, value_text) = arguments
        .split_once(char::is_whitespace)
        .ok_or("Usage: /spreadsheet-set <A1> <text|number|boolean|blank> <value>.")?;
    let (row, column) = parse_coordinate(address)?;
    let value = if formula_only {
        let expression = value_text.trim();
        if !expression.starts_with('=') {
            return Err("A formula must begin with '='.");
        }
        scriberig_domain_spreadsheet::CellValue::Formula(scriberig_domain_spreadsheet::Formula {
            expression: expression.to_owned(),
            cached_value: None,
        })
    } else {
        let mut pieces = value_text.trim().splitn(2, char::is_whitespace);
        let kind = pieces.next().unwrap_or_default();
        let authored = pieces.next().unwrap_or_default().trim();
        match kind {
            "text" if !authored.is_empty() => {
                scriberig_domain_spreadsheet::CellValue::Text(authored.to_owned())
            }
            "number" if !authored.is_empty() => {
                scriberig_domain_spreadsheet::CellValue::Number(authored.to_owned())
            }
            "boolean" if authored == "true" => {
                scriberig_domain_spreadsheet::CellValue::Boolean(true)
            }
            "boolean" if authored == "false" => {
                scriberig_domain_spreadsheet::CellValue::Boolean(false)
            }
            "blank" if authored.is_empty() => scriberig_domain_spreadsheet::CellValue::Blank,
            "text" | "number" => return Err("Text and number values must include content."),
            "boolean" => return Err("Boolean values must be `true` or `false`."),
            "blank" => return Err("Blank values do not accept content."),
            _ => return Err("Use one of: text, number, boolean, or blank."),
        }
    };
    Ok((row, column, value))
}

/// Parses one inclusive A1-style rectangular range.
///
/// # Parameters
/// - `text`: Range such as `A1:C20`.
///
/// # Returns
/// One-based inclusive row and column bounds.
///
/// # Errors
/// Returns an error for malformed, zero-based, or reversed coordinates.
fn parse_range(text: &str) -> Result<scriberig_domain_spreadsheet::CellRange, &'static str> {
    let Some((start, end)) = text.split_once(':') else {
        return Err("A worksheet range must use A1:C20 notation.");
    };
    let (first_row, first_column) = parse_coordinate(start)?;
    let (last_row, last_column) = parse_coordinate(end)?;
    if first_row > last_row || first_column > last_column {
        return Err("Worksheet range coordinates must be ordered from top-left to bottom-right.");
    }
    Ok(scriberig_domain_spreadsheet::CellRange {
        first_row,
        first_column,
        last_row,
        last_column,
    })
}

/// Parses one A1-style cell coordinate.
///
/// # Parameters
/// - `text`: One coordinate such as `AB12`.
///
/// # Returns
/// One-based row and column numbers.
///
/// # Errors
/// Returns an error when the coordinate is malformed or exceeds `u32`.
pub(super) fn parse_coordinate(text: &str) -> Result<(u32, u32), &'static str> {
    let split = text
        .find(|character: char| character.is_ascii_digit())
        .ok_or("Worksheet coordinates must include a column and row, such as A1:C20.")?;
    let (letters, digits) = text.split_at(split);
    if letters.is_empty()
        || !letters
            .chars()
            .all(|character| character.is_ascii_alphabetic())
        || digits.is_empty()
        || !digits.chars().all(|character| character.is_ascii_digit())
    {
        return Err("Worksheet coordinates must use A1 notation.");
    }
    let column = letters.chars().try_fold(0_u32, |value, character| {
        value
            .checked_mul(26)?
            .checked_add(character.to_ascii_uppercase() as u32 - 'A' as u32 + 1)
    });
    let row = digits
        .parse::<u32>()
        .map_err(|_| "Worksheet row number is too large.")?;
    let column = column.ok_or("Worksheet column is too large.")?;
    if row == 0 || column == 0 {
        return Err("Worksheet rows and columns start at 1.");
    }
    if row > 1_048_576 || column > 16_384 {
        return Err("Worksheet coordinates exceed the XLSX row or column limits.");
    }
    Ok((row, column))
}

#[cfg(test)]
mod tests {
    use super::{parse, parse_edit};

    /// Parses multi-letter columns and the optional offset without losing coordinate precision.
    #[test]
    fn parses_a1_range_and_offset() {
        let (range, offset) = parse("AA4:AC12 80").expect("valid range");
        let range = range.expect("range supplied");
        assert_eq!((range.first_row, range.first_column), (4, 27));
        assert_eq!((range.last_row, range.last_column), (12, 29));
        assert_eq!(offset, 80);
    }

    /// Rejects zero-based, reversed, and malformed range arguments.
    #[test]
    fn rejects_invalid_ranges() {
        assert!(parse("A0:B2").is_err());
        assert!(parse("B2:A1").is_err());
        assert!(parse("A1:B2 extra").is_err());
    }

    /// Parses typed cell values and formulas without converting numbers through floats.
    #[test]
    fn parses_exact_value_and_formula_candidates() {
        use scriberig_domain_spreadsheet::CellValue;
        let (row, column, value) = parse_edit("B3 number 12.3400", false).expect("number edit");
        assert_eq!((row, column), (3, 2));
        assert_eq!(value, CellValue::Number("12.3400".to_owned()));
        let (_, _, value) = parse_edit("C4 =SUM(A1:A3)", true).expect("formula edit");
        assert_eq!(
            value,
            CellValue::Formula(scriberig_domain_spreadsheet::Formula {
                expression: "=SUM(A1:A3)".to_owned(),
                cached_value: None,
            })
        );
        assert!(parse_edit("C4 SUM(A1:A3)", true).is_err());
    }
}
