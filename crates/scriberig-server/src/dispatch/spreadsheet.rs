//! Maps typed spreadsheet read requests through trusted application context.

use std::str::FromStr;

use scriberig_app::{ApplicationContext, CapabilityContext, Principal};
use scriberig_core::SnapshotId;
use scriberig_domain_spreadsheet::{Cell, CellValue};
use scriberig_protocol::{
    ProtocolError, ProtocolErrorCode, Response, SpreadsheetCandidateValidation,
    SpreadsheetCellInspectRequest, SpreadsheetCellInspectResponse, SpreadsheetCellStyle,
    SpreadsheetCellValue, SpreadsheetCellView, SpreadsheetCellsRequest, SpreadsheetCellsResponse,
    SpreadsheetEditRequest, SpreadsheetEditResponse, SpreadsheetRenameRequest,
    SpreadsheetTableSummary, SpreadsheetWorkbookRequest, SpreadsheetWorkbookResponse,
    SpreadsheetWorksheetSummary,
};

use crate::server::ApplicationService;

#[path = "spreadsheet_find.rs"]
mod find;
pub(super) use find::find_cells;
#[path = "spreadsheet_range_edit.rs"]
mod range_edit;
pub(super) use range_edit::propose_range;

/// Inspects one cell coordinate using host-injected snapshot and Inspect authority.
///
/// # Parameters
/// - `service`: Shared application boundary.
/// - `capabilities`: Grants resolved by the trusted host.
/// - `principal`: Authenticated caller established by the transport.
/// - `request`: Exact-snapshot worksheet and coordinate selection.
/// - `correlation_id`: Protocol request tracing identity.
///
/// # Returns
/// Typed cell data or an explicit sparse blank coordinate.
pub(super) fn inspect_cell<S: ApplicationService>(
    service: &S,
    capabilities: &CapabilityContext,
    principal: Option<&Principal>,
    request: &SpreadsheetCellInspectRequest,
    correlation_id: &str,
) -> Response {
    let Ok(snapshot) = SnapshotId::from_str(&request.snapshot) else {
        return schema_error("invalid snapshot identity", correlation_id);
    };
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        principal: principal.cloned(),
        capabilities: capabilities.clone(),
        correlation_id: Some(correlation_id.to_owned()),
        ..ApplicationContext::default()
    };
    service
        .inspect_spreadsheet_cell(
            &context,
            &request.project,
            &request.worksheet,
            request.row,
            request.column,
        )
        .map_or_else(
            |error| Response::Error(super::helpers::map_error(&error, correlation_id)),
            |result| {
                Response::SpreadsheetCellInspect(SpreadsheetCellInspectResponse {
                    project: result.project.to_string(),
                    snapshot: result.snapshot.to_string(),
                    worksheet: result.worksheet.to_string(),
                    row: result.row,
                    column: result.column,
                    cell: result.cell.map(map_cell),
                })
            },
        )
}

/// Reads a workbook summary using host-injected snapshot and Inspect authority.
///
/// # Parameters
/// - `service`: Shared application boundary.
/// - `capabilities`: Grants resolved by the trusted host.
/// - `principal`: Authenticated caller established by the transport.
/// - `request`: Exact-snapshot workbook selection.
/// - `correlation_id`: Protocol request tracing identity.
///
/// # Returns
/// Typed workbook metadata or a structured application error.
pub(super) fn inspect_workbook<S: ApplicationService>(
    service: &S,
    capabilities: &CapabilityContext,
    principal: Option<&Principal>,
    request: SpreadsheetWorkbookRequest,
    correlation_id: &str,
) -> Response {
    let Ok(snapshot) = SnapshotId::from_str(&request.snapshot) else {
        return schema_error("invalid snapshot identity", correlation_id);
    };
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        principal: principal.cloned(),
        capabilities: capabilities.clone(),
        correlation_id: Some(correlation_id.to_owned()),
        ..ApplicationContext::default()
    };
    service
        .inspect_spreadsheet_workbook(&context, &request.project, &request.workbook)
        .map_or_else(
            |error| Response::Error(super::helpers::map_error(&error, correlation_id)),
            |summary| {
                Response::SpreadsheetWorkbook(SpreadsheetWorkbookResponse {
                    project: request.project,
                    snapshot: summary.snapshot.to_string(),
                    workbook: summary.id.to_string(),
                    artifact_capability: summary
                        .artifact_capability
                        .map(super::helpers::map_artifact_edit_capability),
                    worksheets: summary
                        .worksheets
                        .into_iter()
                        .map(|worksheet| SpreadsheetWorksheetSummary {
                            id: worksheet.id.to_string(),
                            name: worksheet.name,
                            used_range: worksheet.used_range.map(|range| {
                                scriberig_protocol::SpreadsheetRange {
                                    first_row: range.first_row,
                                    first_column: range.first_column,
                                    last_row: range.last_row,
                                    last_column: range.last_column,
                                }
                            }),
                            tables: worksheet
                                .tables
                                .into_iter()
                                .map(|table| SpreadsheetTableSummary {
                                    id: table.id.to_string(),
                                    name: table.name,
                                    range: scriberig_protocol::SpreadsheetRange {
                                        first_row: table.range.first_row,
                                        first_column: table.range.first_column,
                                        last_row: table.range.last_row,
                                        last_column: table.range.last_column,
                                    },
                                })
                                .collect(),
                        })
                        .collect(),
                })
            },
        )
}

/// Reads one sparse worksheet page using host-injected scoped grants.
///
/// - service: Application boundary receiving the trusted request context.
/// - capabilities: Capability grants resolved by the host, never by the client.
/// - principal: Authenticated caller established by the transport boundary.
/// - request: Exact snapshot and bounded page selection.
/// - `correlation_id`: Protocol request tracing identity.
///
/// Typed spreadsheet page or a structured request/application error.
pub(super) fn read<S: ApplicationService>(
    service: &S,
    capabilities: &CapabilityContext,
    principal: Option<&Principal>,
    request: SpreadsheetCellsRequest,
    correlation_id: &str,
) -> Response {
    let Ok(snapshot) = SnapshotId::from_str(&request.snapshot) else {
        return schema_error("invalid snapshot identity", correlation_id);
    };
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        principal: principal.cloned(),
        capabilities: capabilities.clone(),
        correlation_id: Some(correlation_id.to_owned()),
        ..ApplicationContext::default()
    };
    let range = request
        .range
        .map(|range| scriberig_protocol::SpreadsheetRange {
            first_row: range.first_row,
            first_column: range.first_column,
            last_row: range.last_row,
            last_column: range.last_column,
        });
    service
        .read_spreadsheet_cells(
            &context,
            &request.project,
            &request.worksheet,
            range,
            request.offset,
            request.limit,
        )
        .map_or_else(
            |error| Response::Error(super::helpers::map_error(&error, correlation_id)),
            |result| {
                Response::SpreadsheetCells(SpreadsheetCellsResponse {
                    project: result.project.to_string(),
                    snapshot: result.snapshot.to_string(),
                    worksheet: result.page.worksheet.to_string(),
                    artifact_capability: result
                        .page
                        .artifact_capability
                        .map(super::helpers::map_artifact_edit_capability),
                    artifact_diagnostic: result.page.artifact_diagnostic,
                    cells: result.page.cells.into_iter().map(map_cell).collect(),
                    total: result.page.total,
                    has_more: result.page.has_more,
                })
            },
        )
}

/// Proposes an exact-base semantic cell edit using host-injected authority.
///
/// - service: Shared application boundary.
/// - capabilities: Grants resolved by the trusted host.
/// - principal: Authenticated caller established by the transport.
/// - request: Exact base and typed target edit.
/// - `correlation_id`: Request tracing identity.
///
/// Reviewable candidate handle or structured application error.
pub(super) fn propose<S: ApplicationService>(
    service: &S,
    capabilities: &CapabilityContext,
    principal: Option<&Principal>,
    request: SpreadsheetEditRequest,
    correlation_id: &str,
) -> Response {
    let Ok(snapshot) = SnapshotId::from_str(&request.snapshot) else {
        return schema_error("invalid snapshot identity", correlation_id);
    };
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        principal: principal.cloned(),
        capabilities: capabilities.clone(),
        correlation_id: Some(correlation_id.to_owned()),
        ..ApplicationContext::default()
    };
    service
        .propose_spreadsheet_cell_edit(
            &context,
            &request.project,
            &request.worksheet,
            request.row,
            request.column,
            request.value,
        )
        .map_or_else(
            |error| Response::Error(super::helpers::map_error(&error, correlation_id)),
            |candidate| {
                let validation = match candidate.validation {
                    scriberig_project::CandidateValidation::Pending => {
                        SpreadsheetCandidateValidation::Pending
                    }
                    scriberig_project::CandidateValidation::Passed => {
                        SpreadsheetCandidateValidation::Passed
                    }
                    scriberig_project::CandidateValidation::Failed { diagnostics } => {
                        SpreadsheetCandidateValidation::Failed { diagnostics }
                    }
                };
                Response::SpreadsheetEdit(SpreadsheetEditResponse {
                    project: candidate.project.to_string(),
                    candidate_id: candidate.id,
                    base_snapshot: candidate.base_snapshot.to_string(),
                    path: candidate.path,
                    validation,
                })
            },
        )
}

/// Proposes an exact-base worksheet rename using host-injected Propose authority.
///
/// # Parameters
/// - `service`: Shared application boundary.
/// - `capabilities`: Grants resolved by the trusted host.
/// - `principal`: Authenticated caller established by the transport.
/// - `request`: Exact base and typed worksheet rename.
/// - `correlation_id`: Request tracing identity.
///
/// # Returns
/// Reviewable candidate handle or structured application error.
pub(super) fn rename_sheet<S: ApplicationService>(
    service: &S,
    capabilities: &CapabilityContext,
    principal: Option<&Principal>,
    request: &SpreadsheetRenameRequest,
    correlation_id: &str,
) -> Response {
    let Ok(snapshot) = SnapshotId::from_str(&request.snapshot) else {
        return schema_error("invalid snapshot identity", correlation_id);
    };
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        principal: principal.cloned(),
        capabilities: capabilities.clone(),
        correlation_id: Some(correlation_id.to_owned()),
        ..ApplicationContext::default()
    };
    service
        .propose_spreadsheet_rename(
            &context,
            &request.project,
            &request.worksheet,
            &request.new_name,
        )
        .map_or_else(
            |error| Response::Error(super::helpers::map_error(&error, correlation_id)),
            |candidate| {
                let validation = match candidate.validation {
                    scriberig_project::CandidateValidation::Pending => {
                        SpreadsheetCandidateValidation::Pending
                    }
                    scriberig_project::CandidateValidation::Passed => {
                        SpreadsheetCandidateValidation::Passed
                    }
                    scriberig_project::CandidateValidation::Failed { diagnostics } => {
                        SpreadsheetCandidateValidation::Failed { diagnostics }
                    }
                };
                Response::SpreadsheetRename(SpreadsheetEditResponse {
                    project: candidate.project.to_string(),
                    candidate_id: candidate.id,
                    base_snapshot: candidate.base_snapshot.to_string(),
                    path: candidate.path,
                    validation,
                })
            },
        )
}

/// Converts one semantic cell into its stable public view.
///
/// # Parameters
/// - `cell`: Snapshot-bound semantic spreadsheet cell.
///
/// # Returns
/// Wire-safe typed cell with semantic identity and coordinates.
fn map_cell(cell: Cell) -> SpreadsheetCellView {
    SpreadsheetCellView {
        id: cell.id.entity().to_string(),
        row: cell.row,
        column: cell.column,
        value: map_value(cell.value),
        style: cell.style.map(|style| SpreadsheetCellStyle {
            key: style.key,
            adapter: style.adapter,
        }),
    }
}

/// Converts one semantic value without promoting cached formula results.
///
/// # Parameters
/// - `value`: Provider-neutral authored cell value.
///
/// # Returns
/// Typed transport value.
fn map_value(value: CellValue) -> SpreadsheetCellValue {
    match value {
        CellValue::Blank => SpreadsheetCellValue::Blank,
        CellValue::Text(value) => SpreadsheetCellValue::Text(value),
        CellValue::Number(value) => SpreadsheetCellValue::Number(value),
        CellValue::Boolean(value) => SpreadsheetCellValue::Boolean(value),
        CellValue::Error(value) => SpreadsheetCellValue::Error(value),
        CellValue::Formula(formula) => SpreadsheetCellValue::Formula {
            expression: formula.expression,
            cached_value: formula
                .cached_value
                .map(|value| Box::new(map_value(*value))),
        },
    }
}

/// Creates a protocol schema error without exposing request contents.
///
/// # Parameters
/// - `message`: Safe schema diagnostic.
/// - `correlation_id`: Request tracing identity.
///
/// # Returns
/// Structured protocol error response.
pub(super) fn schema_error(message: &str, correlation_id: &str) -> Response {
    Response::Error(ProtocolError {
        code: ProtocolErrorCode::SchemaViolation,
        message: message.to_owned(),
        details: None,
        correlation_id: Some(correlation_id.to_owned()),
    })
}
