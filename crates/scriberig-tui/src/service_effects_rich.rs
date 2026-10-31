//! Executes artifact and rich-domain inspection effects for the TUI.

use super::ApplicationContext;
use crate::{action::Action, effect::Effect, service::TuiService};

/// Executes an effect owned by a rich-domain semantic view.
///
/// # Parameters
/// - `service`: Shared TUI service that owns the operation.
/// - `context`: Authorized application context.
/// - `effect`: Effect to execute.
///
/// # Returns
/// The resulting action when this module owns the effect, or the unchanged
/// effect when another service-effects module owns it.
///
/// # Errors
/// Returns the effect unchanged when no rich-domain handler owns it.
pub(super) fn execute<S: TuiService>(
    service: &S,
    context: &ApplicationContext,
    effect: Effect,
) -> Result<Action, Box<Effect>> {
    let effect = match inspection::execute_inspection(service, context, effect) {
        Ok(action) => return Ok(action),
        Err(effect) => *effect,
    };
    let effect = match document_spreadsheet_cad::execute_document_spreadsheet_cad(
        service, context, effect,
    ) {
        Ok(action) => return Ok(action),
        Err(effect) => *effect,
    };
    let effect = match qgis_style::execute_qgis_style(service, context, effect) {
        Ok(action) => return Ok(action),
        Err(effect) => *effect,
    };
    let effect = match geo_and_edits::execute_geo_and_edits(service, context, effect) {
        Ok(action) => return Ok(action),
        Err(effect) => *effect,
    };
    Err(Box::new(effect))
}

#[path = "service_effects_rich/document_spreadsheet_cad.rs"]
mod document_spreadsheet_cad;
#[path = "service_effects_rich/geo_and_edits.rs"]
mod geo_and_edits;
#[path = "service_effects_rich/inspection.rs"]
mod inspection;
#[path = "service_effects_rich/qgis_style.rs"]
mod qgis_style;
