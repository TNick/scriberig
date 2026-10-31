//! Normalizes local spreadsheet reads to the public protocol view model.

/// Converts an authorized application page into the same typed model used remotely.
///
/// # Parameters
/// - `page`: Bounded exact-snapshot application cell page.
///
/// # Returns
/// A protocol-shaped page with semantic identities and typed values preserved.
pub fn from_application(
    page: scriberig_app::SpreadsheetCells,
) -> scriberig_protocol::SpreadsheetCellsResponse {
    let capability = page.page.artifact_capability.map(|value| match value {
        scriberig_project::ArtifactCapability::Unsupported => {
            scriberig_protocol::ArtifactEditCapability::Unsupported
        }
        scriberig_project::ArtifactCapability::InspectOnly => {
            scriberig_protocol::ArtifactEditCapability::InspectOnly
        }
        scriberig_project::ArtifactCapability::ReadOnlyDerived => {
            scriberig_protocol::ArtifactEditCapability::ReadOnlyDerived
        }
        scriberig_project::ArtifactCapability::LosslessEditable => {
            scriberig_protocol::ArtifactEditCapability::LosslessEditable
        }
        scriberig_project::ArtifactCapability::LossAwareEditable => {
            scriberig_protocol::ArtifactEditCapability::LossAwareEditable
        }
        scriberig_project::ArtifactCapability::EditableWithOpaquePreservation => {
            scriberig_protocol::ArtifactEditCapability::EditableWithOpaquePreservation
        }
        scriberig_project::ArtifactCapability::RestrictedExternalProviderEdit => {
            scriberig_protocol::ArtifactEditCapability::RestrictedExternalProviderEdit
        }
    });
    scriberig_protocol::SpreadsheetCellsResponse {
        project: page.project.to_string(),
        snapshot: page.snapshot.to_string(),
        worksheet: page.page.worksheet.to_string(),
        artifact_capability: capability,
        artifact_diagnostic: page.page.artifact_diagnostic,
        cells: page
            .page
            .cells
            .into_iter()
            .map(cell_from_application)
            .collect(),
        total: page.page.total,
        has_more: page.page.has_more,
    }
}

/// Converts one application coordinate inspection into the public typed response.
///
/// # Parameters
/// - `cell`: Exact-snapshot coordinate result, including absent blank cells.
///
/// # Returns
/// Protocol-shaped coordinate inspection with exact identities retained.
pub fn cell_inspection_from_application(
    cell: scriberig_app::SpreadsheetCellInspection,
) -> scriberig_protocol::SpreadsheetCellInspectResponse {
    scriberig_protocol::SpreadsheetCellInspectResponse {
        project: cell.project.to_string(),
        snapshot: cell.snapshot.to_string(),
        worksheet: cell.worksheet.to_string(),
        row: cell.row,
        column: cell.column,
        cell: cell.cell.map(cell_from_application),
    }
}

/// Converts one semantic cell without promoting its cached formula value.
///
/// # Parameters
/// - `cell`: Snapshot-resolved semantic cell.
///
/// # Returns
/// Protocol cell with stable identity, coordinates, typed value, and style.
fn cell_from_application(
    cell: scriberig_domain_spreadsheet::Cell,
) -> scriberig_protocol::SpreadsheetCellView {
    scriberig_protocol::SpreadsheetCellView {
        id: cell.id.entity().to_string(),
        row: cell.row,
        column: cell.column,
        value: value_from_application(cell.value),
        style: cell
            .style
            .map(|style| scriberig_protocol::SpreadsheetCellStyle {
                key: style.key,
                adapter: style.adapter,
            }),
    }
}

/// Converts an authored scalar or formula value recursively.
///
/// # Parameters
/// - `value`: Semantic domain value.
///
/// # Returns
/// Equivalent wire value preserving formula and cached-result separation.
#[must_use]
pub fn value_from_application(
    value: scriberig_domain_spreadsheet::CellValue,
) -> scriberig_protocol::SpreadsheetCellValue {
    use scriberig_domain_spreadsheet::CellValue as Domain;
    use scriberig_protocol::SpreadsheetCellValue as Wire;
    match value {
        Domain::Blank => Wire::Blank,
        Domain::Text(value) => Wire::Text(value),
        Domain::Number(value) => Wire::Number(value),
        Domain::Boolean(value) => Wire::Boolean(value),
        Domain::Error(value) => Wire::Error(value),
        Domain::Formula(formula) => Wire::Formula {
            expression: formula.expression,
            cached_value: formula
                .cached_value
                .map(|value| Box::new(value_from_application(*value))),
        },
    }
}
