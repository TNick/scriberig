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
pub(super) fn execute_qgis_style<S: TuiService>(
    service: &S,
    context: &ApplicationContext,
    effect: Effect,
) -> Result<Action, Box<Effect>> {
    Ok(match effect {
        Effect::InspectGeoCatalog {
            request_id,
            project,
            path,
            snapshot,
            offset,
        } => service
            .inspect_geopackage(context, project, &path, snapshot, offset)
            .map(|catalog| Action::GeoCatalogLoaded {
                request_id,
                catalog,
            })
            .unwrap_or_else(|error| Action::GeoCatalogFailed {
                request_id,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::InspectQgisProject {
            request_id,
            request,
        } => service
            .inspect_qgis_project(context, &request)
            .map(|project| Action::QgisProjectLoaded {
                request_id,
                project,
            })
            .unwrap_or_else(|error| Action::QgisProjectFailed {
                request_id,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::ProposeQgisLayerRename {
            request_id,
            request,
        } => service
            .propose_qgis_layer_rename(context, &request)
            .map(|candidate| Action::QgisLayerRenameProposed {
                request_id,
                candidate,
            })
            .unwrap_or_else(|error| Action::QgisLayerRenameFailed {
                request_id,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::ProposeQgisLayerOrder {
            request_id,
            request,
        } => service
            .propose_qgis_layer_order(context, &request)
            .map(|candidate| Action::QgisLayerOrderProposed {
                request_id,
                candidate,
            })
            .unwrap_or_else(|error| Action::QgisLayerOrderFailed {
                request_id,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::SetProjectionStylePreference {
            request_id,
            request,
        } => service
            .set_projection_style_preference(context, &request)
            .map(|setting| Action::ProjectionStylePreferenceSet {
                request_id,
                setting,
            })
            .unwrap_or_else(|error| Action::ProjectionStylePreferenceFailed {
                request_id,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::ResolveProjectionStyle {
            request_id,
            request,
        } => service
            .resolve_projection_style(context, &request)
            .map(|resolution| Action::ProjectionStyleResolved {
                request_id,
                resolution,
            })
            .unwrap_or_else(|error| Action::ProjectionStyleResolveFailed {
                request_id,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        other => return Err(Box::new(other)),
    })
}
