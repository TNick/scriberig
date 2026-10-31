//! Routes spreadsheet semantic reads and candidate commands.

use super::*;

/// Routes a supported spreadsheet command to a snapshot-bound reducer action.
///
/// # Parameters
/// - `state`: Current reducer state and worksheet selection.
/// - `command`: Full submitted command text.
/// - `name`: Command name parsed by the shared descriptor registry.
/// - `command_name`: Canonical registered command identity.
///
/// # Returns
/// The resulting reducer effect, or no result for an unrelated command.
pub(super) fn dispatch(
    state: &mut TuiState,
    command: &str,
    name: &str,
    command_name: &str,
) -> Option<Vec<Effect>> {
    match command_name {
        "/spreadsheet" => {
            let (project, snapshot, worksheet) = match spreadsheet_command::scope(state) {
                Ok(scope) => scope,
                Err(message) => {
                    state.conversation_composer.error = Some(message.to_owned());
                    return Some(vec![Effect::None]);
                }
            };
            let args = command.strip_prefix(name).unwrap_or_default().trim();
            let (range, offset) = match spreadsheet_command::parse(args) {
                Ok(value) => value,
                Err(message) => {
                    state.conversation_composer.error = Some(message.to_owned());
                    return Some(vec![Effect::None]);
                }
            };
            Some(super::reduce(
                state,
                Action::SpreadsheetRequested {
                    project,
                    snapshot,
                    worksheet,
                    range,
                    offset,
                },
            ))
        }
        "/spreadsheet-cell" => {
            let (project, snapshot, worksheet) = match spreadsheet_command::scope(state) {
                Ok(scope) => scope,
                Err(message) => {
                    state.conversation_composer.error = Some(message.to_owned());
                    return Some(vec![Effect::None]);
                }
            };
            let coordinate = command.strip_prefix(name).unwrap_or_default().trim();
            let (row, column) = match spreadsheet_command::parse_coordinate(coordinate) {
                Ok(value) => value,
                Err(message) => {
                    state.conversation_composer.error = Some(message.to_owned());
                    return Some(vec![Effect::None]);
                }
            };
            Some(super::reduce(
                state,
                Action::SpreadsheetCellRequested(scriberig_app::SpreadsheetCellQuery {
                    project,
                    snapshot,
                    worksheet,
                    row,
                    column,
                }),
            ))
        }
        "/spreadsheet-set" | "/spreadsheet-formula" => {
            let (project, snapshot, worksheet) = match spreadsheet_command::scope(state) {
                Ok(scope) => scope,
                Err(message) => {
                    state.conversation_composer.error = Some(message.to_owned());
                    return Some(vec![Effect::None]);
                }
            };
            let args = command.strip_prefix(name).unwrap_or_default().trim();
            let formula_only = command_name == "/spreadsheet-formula";
            let (row, column, value) = match spreadsheet_command::parse_edit(args, formula_only) {
                Ok(value) => value,
                Err(message) => {
                    state.conversation_composer.error = Some(message.to_owned());
                    return Some(vec![Effect::None]);
                }
            };
            Some(super::reduce(
                state,
                Action::SpreadsheetEditRequested(scriberig_app::SpreadsheetCellEditQuery {
                    project,
                    snapshot,
                    worksheet,
                    row,
                    column,
                    value,
                }),
            ))
        }
        _ => None,
    }
}
