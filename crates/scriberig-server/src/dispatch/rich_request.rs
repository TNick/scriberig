//! Routes rich artifact protocol requests through the application dispatcher.

use super::{Dispatcher, artifacts, cad, cad_bounds, cad_find, document, geo, qgis, spreadsheet};
use crate::server::ApplicationService;
use scriberig_protocol::{Request, Response};

/// Dispatches rich artifact requests and leaves unrelated requests to the general router.
///
/// # Parameters
/// - `dispatcher`: Host-configured application boundary and trusted grants.
/// - `request`: Version-validated protocol request.
/// - `correlation_id`: Request tracing identity.
///
/// # Returns
/// A protocol response when the request belongs to a rich artifact surface, or `None` otherwise.
#[allow(
    clippy::too_many_lines,
    reason = "This dispatcher is a declarative routing table; individual handlers live in the extracted request modules."
)]
pub(super) fn dispatch<S: ApplicationService>(
    dispatcher: &Dispatcher<S>,
    request: &Request,
    correlation_id: &str,
) -> Option<Response> {
    let response = match request {
        Request::CadDrawingInspect(request) => cad::inspect(
            &dispatcher.service,
            &dispatcher.capabilities,
            dispatcher.principal.as_ref(),
            request,
            correlation_id,
        ),
        Request::CadEntities(request) => cad::read_entities(
            &dispatcher.service,
            &dispatcher.capabilities,
            dispatcher.principal.as_ref(),
            request,
            correlation_id,
        ),
        Request::CadTextFind(request) => cad_find::find_text(
            &dispatcher.service,
            &dispatcher.capabilities,
            dispatcher.principal.as_ref(),
            request,
            correlation_id,
        ),
        Request::CadBoundsQuery(request) => cad_bounds::query(
            &dispatcher.service,
            &dispatcher.capabilities,
            dispatcher.principal.as_ref(),
            request.clone(),
            correlation_id,
        ),
        Request::CadLineEdit(request) => cad::propose(
            &dispatcher.service,
            &dispatcher.capabilities,
            dispatcher.principal.as_ref(),
            request.clone(),
            correlation_id,
        ),
        Request::CadLayerChange(request) => cad::propose_layer_change(
            &dispatcher.service,
            &dispatcher.capabilities,
            dispatcher.principal.as_ref(),
            request,
            correlation_id,
        ),
        Request::ArtifactInspect(request) => artifacts::inspect(
            &dispatcher.service,
            &dispatcher.application_context(None, correlation_id),
            request,
            correlation_id,
        ),
        Request::ArtifactMembers(request) => artifacts::members(
            &dispatcher.service,
            &dispatcher.application_context(None, correlation_id),
            request,
            correlation_id,
        ),
        Request::SpreadsheetCells(request) => spreadsheet::read(
            &dispatcher.service,
            &dispatcher.capabilities,
            dispatcher.principal.as_ref(),
            request.clone(),
            correlation_id,
        ),
        Request::SpreadsheetCellInspect(request) => spreadsheet::inspect_cell(
            &dispatcher.service,
            &dispatcher.capabilities,
            dispatcher.principal.as_ref(),
            request,
            correlation_id,
        ),
        Request::SpreadsheetWorkbook(request) => spreadsheet::inspect_workbook(
            &dispatcher.service,
            &dispatcher.capabilities,
            dispatcher.principal.as_ref(),
            request.clone(),
            correlation_id,
        ),
        Request::SpreadsheetCellFind(request) => spreadsheet::find_cells(
            &dispatcher.service,
            &dispatcher.capabilities,
            dispatcher.principal.as_ref(),
            request,
            correlation_id,
        ),
        Request::SpreadsheetRename(request) => spreadsheet::rename_sheet(
            &dispatcher.service,
            &dispatcher.capabilities,
            dispatcher.principal.as_ref(),
            request,
            correlation_id,
        ),
        Request::GeoFeatures(request) => geo::read(
            &dispatcher.service,
            &dispatcher.capabilities,
            dispatcher.principal.as_ref(),
            request,
            correlation_id,
        ),
        Request::GeoFeatureQuery(request) => geo::query(
            &dispatcher.service,
            &dispatcher.capabilities,
            dispatcher.principal.as_ref(),
            request,
            correlation_id,
        ),
        Request::QgisProject(request) => qgis::inspect(
            &dispatcher.service,
            &dispatcher.capabilities,
            dispatcher.principal.as_ref(),
            request,
            correlation_id,
        ),
        Request::QgisLayerRename(request) => qgis::rename(
            &dispatcher.service,
            &dispatcher.capabilities,
            dispatcher.principal.as_ref(),
            request,
            correlation_id,
        ),
        Request::QgisLayerOrder(request) => qgis::reorder(
            &dispatcher.service,
            &dispatcher.capabilities,
            dispatcher.principal.as_ref(),
            request,
            correlation_id,
        ),
        Request::GeoGeometryEdit(request) => geo::propose_geometry(
            &dispatcher.service,
            &dispatcher.capabilities,
            dispatcher.principal.as_ref(),
            request.clone(),
            correlation_id,
        ),
        Request::GeoAttributeEdit(request) => geo::propose_attribute(
            &dispatcher.service,
            &dispatcher.capabilities,
            dispatcher.principal.as_ref(),
            request.clone(),
            correlation_id,
        ),
        Request::GeoCatalog(request) => geo::inspect(
            &dispatcher.service,
            &dispatcher.capabilities,
            dispatcher.principal.as_ref(),
            request,
            correlation_id,
        ),
        Request::SpreadsheetEdit(request) => spreadsheet::propose(
            &dispatcher.service,
            &dispatcher.capabilities,
            dispatcher.principal.as_ref(),
            request.clone(),
            correlation_id,
        ),
        Request::SpreadsheetRangeEdit(request) => spreadsheet::propose_range(
            &dispatcher.service,
            &dispatcher.capabilities,
            dispatcher.principal.as_ref(),
            request.clone(),
            correlation_id,
        ),
        Request::DocumentParagraphEdit(request) => document::propose(
            &dispatcher.service,
            &dispatcher.capabilities,
            dispatcher.principal.as_ref(),
            request,
            correlation_id,
        ),
        Request::DocumentTableCellEdit(request) => document::propose_table_cell(
            &dispatcher.service,
            &dispatcher.capabilities,
            dispatcher.principal.as_ref(),
            request,
            correlation_id,
        ),
        Request::DocumentParagraphInsert(request) => document::insert_paragraph(
            &dispatcher.service,
            &dispatcher.capabilities,
            dispatcher.principal.as_ref(),
            request,
            correlation_id,
        ),
        Request::DocumentInspect(request) => document::inspect(
            &dispatcher.service,
            &dispatcher.capabilities,
            dispatcher.principal.as_ref(),
            request,
            correlation_id,
        ),
        Request::DocumentParagraphFind(request) => document::find_paragraphs(
            &dispatcher.service,
            &dispatcher.capabilities,
            dispatcher.principal.as_ref(),
            request,
            correlation_id,
        ),
        _ => return None,
    };
    Some(response)
}
