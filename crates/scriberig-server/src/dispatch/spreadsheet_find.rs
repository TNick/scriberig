//! Dispatches exact-snapshot spreadsheet cell searches through the app service.

use std::str::FromStr;

use scriberig_app::{ApplicationContext, CapabilityContext, Principal};
use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_protocol::{
    Response, SpreadsheetCellFindRequest, SpreadsheetCellFindResponse, SpreadsheetCellMatch,
};

use crate::server::ApplicationService;

use super::schema_error;

/// Finds authored cell values with host-injected exact-snapshot Inspect authority.
///
/// # Parameters
/// - `service`: Shared application boundary.
/// - `capabilities`: Grants resolved by the trusted host.
/// - `principal`: Authenticated caller established by the transport.
/// - `request`: Exact-snapshot workbook search and page selection.
/// - `correlation_id`: Protocol request tracing identity.
///
/// # Returns
/// Bounded semantic cell matches or a structured application error.
pub(crate) fn find_cells<S: ApplicationService>(
    service: &S,
    capabilities: &CapabilityContext,
    principal: Option<&Principal>,
    request: &SpreadsheetCellFindRequest,
    correlation_id: &str,
) -> Response {
    let Ok(project) = ProjectId::from_str(&request.project) else {
        return schema_error("invalid project identity", correlation_id);
    };
    let Ok(snapshot) = SnapshotId::from_str(&request.snapshot) else {
        return schema_error("invalid snapshot identity", correlation_id);
    };
    let Ok(workbook) = EntityId::from_str(&request.workbook) else {
        return schema_error("invalid workbook identity", correlation_id);
    };
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        principal: principal.cloned(),
        capabilities: capabilities.clone(),
        correlation_id: Some(correlation_id.to_owned()),
        ..ApplicationContext::default()
    };
    let search = scriberig_app::SpreadsheetCellSearchRequest {
        project,
        snapshot,
        workbook,
        query: request.query.clone(),
        offset: request.offset,
        limit: request.limit,
    };
    service
        .find_spreadsheet_cells(&context, &search)
        .map_or_else(
            |error| Response::Error(super::super::helpers::map_error(&error, correlation_id)),
            |result| {
                Response::SpreadsheetCellFind(SpreadsheetCellFindResponse {
                    project: result.project.to_string(),
                    snapshot: result.snapshot.to_string(),
                    workbook: result.workbook.to_string(),
                    matches: result
                        .matches
                        .into_iter()
                        .map(|cell| SpreadsheetCellMatch {
                            worksheet: cell.worksheet.to_string(),
                            cell: cell.cell.to_string(),
                            row: cell.row,
                            column: cell.column,
                            authored_value: cell.authored_value,
                        })
                        .collect(),
                    total: result.total,
                    has_more: result.has_more,
                })
            },
        )
}
