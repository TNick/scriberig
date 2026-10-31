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
pub(super) fn execute_geo_and_edits<S: TuiService>(
    service: &S,
    context: &ApplicationContext,
    effect: Effect,
) -> Result<Action, Box<Effect>> {
    Ok(match effect {
        Effect::InspectGeoFeatures {
            request_id,
            project,
            path,
            snapshot,
            layer,
            offset,
        } => service
            .read_geopackage_features(context, project, &path, snapshot, layer, offset)
            .map(|page| Action::GeoFeaturesLoaded { request_id, page })
            .unwrap_or_else(|error| Action::GeoFeaturesFailed {
                request_id,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::QueryGeoFeatures {
            request_id,
            request,
        } => service
            .query_geospatial_features(context, &request)
            .map(|result| Action::GeoFeatureQueryLoaded { request_id, result })
            .unwrap_or_else(|error| Action::GeoFeatureQueryFailed {
                request_id,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::ProposeGeoAttributeEdit { request_id, edit } => service
            .propose_geo_attribute_edit(context, &edit)
            .map(|candidate| Action::GeoAttributeEditProposed {
                request_id,
                candidate,
            })
            .unwrap_or_else(|error| Action::GeoAttributeEditFailed {
                request_id,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::ProposeGeoGeometryEdit {
            request_id,
            request,
        } => service
            .propose_geo_geometry_edit(context, &request)
            .map(|candidate| Action::GeoGeometryEditProposed {
                request_id,
                candidate,
            })
            .unwrap_or_else(|error| Action::GeoGeometryEditFailed {
                request_id,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::ProposeDocumentParagraphEdit {
            request_id,
            request,
        } => service
            .propose_document_paragraph_edit(context, &request)
            .map(|candidate| Action::DocumentEditProposed {
                request_id,
                candidate,
            })
            .unwrap_or_else(|error| Action::DocumentEditFailed {
                request_id,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::ProposeDocumentTableCellEdit {
            request_id,
            request,
        } => service
            .propose_document_table_cell_edit(context, &request)
            .map(|candidate| Action::DocumentCellEditProposed {
                request_id,
                candidate,
            })
            .unwrap_or_else(|error| Action::DocumentCellEditFailed {
                request_id,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::ProposeSpreadsheetEdit {
            request_id,
            request,
        } => service
            .propose_spreadsheet_edit(context, &request)
            .map(|candidate| Action::SpreadsheetEditProposed {
                request_id,
                candidate,
            })
            .unwrap_or_else(|error| Action::SpreadsheetEditFailed {
                request_id,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        other => return Err(Box::new(other)),
    })
}
