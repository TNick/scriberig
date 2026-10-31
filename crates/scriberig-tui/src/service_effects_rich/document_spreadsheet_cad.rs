//! Executes bounded rich TUI operations for one semantic view family.

use super::super::ApplicationContext;
use crate::{action::Action, effect::Effect, service::TuiService};

/// Executes one bounded group of related semantic view operations.
///
/// # Parameters
/// - `service`: Shared TUI service that owns the operation.
/// - `context`: Authorized application context.
/// - `effect`: Candidate effect to dispatch.
///
/// # Returns
/// The resulting action when this group owns the effect.
///
/// # Errors
/// Returns the effect boxed when another handler owns it.
pub(super) fn execute_document_spreadsheet_cad<S: TuiService>(
    service: &S,
    context: &ApplicationContext,
    effect: Effect,
) -> Result<Action, Box<Effect>> {
    Ok(match effect {
        Effect::InspectCadPage {
            request_id,
            drawing,
            entities,
        } => match service.inspect_cad_drawing(context, &drawing) {
            Ok(summary) => service
                .inspect_cad_entities(context, &entities)
                .map(|page| Action::CadLoaded {
                    request_id,
                    drawing: summary,
                    entities: page,
                })
                .unwrap_or_else(|error| Action::CadFailed {
                    request_id,
                    category: error.category().to_owned(),
                    message: crate::inspector::sanitize(&error.to_string()),
                }),
            Err(error) => Action::CadFailed {
                request_id,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            },
        },
        Effect::ProposeCadLayerChange {
            request_id,
            request,
        } => service
            .propose_cad_layer_change(context, &request)
            .map(|candidate| Action::CadLayerChangeProposed {
                request_id,
                candidate,
            })
            .unwrap_or_else(|error| Action::CadLayerChangeFailed {
                request_id,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::InspectSpreadsheetPage {
            request_id,
            project,
            snapshot,
            worksheet,
            range,
            offset,
            limit,
        } => service
            .inspect_spreadsheet_cells(
                context,
                &scriberig_app::SpreadsheetCellsQuery {
                    project,
                    snapshot,
                    worksheet,
                    range,
                    offset,
                    limit,
                },
            )
            .map(|page| Action::SpreadsheetLoaded { request_id, page })
            .unwrap_or_else(|error| Action::SpreadsheetFailed {
                request_id,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::InspectSpreadsheetCell {
            request_id,
            request,
        } => service
            .inspect_spreadsheet_cell(context, &request)
            .map(|cell| Action::SpreadsheetCellLoaded { request_id, cell })
            .unwrap_or_else(|error| Action::SpreadsheetCellFailed {
                request_id,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::InspectDocument {
            request_id,
            request,
        } => service
            .inspect_document(context, &request)
            .map(|tree| Action::DocumentLoaded { request_id, tree })
            .unwrap_or_else(|error| Action::DocumentFailed {
                request_id,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        other => return Err(Box::new(other)),
    })
}
