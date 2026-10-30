//! Builds exact-snapshot geospatial geometry edit requests.

use crate::{inspector::LoadState, state::TuiState};

/// Builds a typed geometry candidate request for a feature in the loaded exact page.
///
/// # Parameters
/// - `state`: TUI state with exact catalog and feature pages.
/// - `arguments`: Semantic feature ID followed by protocol-shaped geometry JSON.
///
/// # Returns
/// Exact project/snapshot/layer/feature geometry edit request.
///
/// # Errors
/// Returns a safe usage or identity error for targets outside the loaded snapshot.
pub fn geometry_edit(
    state: &TuiState,
    arguments: &str,
) -> Result<scriberig_protocol::GeoGeometryEditRequest, &'static str> {
    let Some((identity, geometry_json)) = arguments.split_once(char::is_whitespace) else {
        return Err("Usage: /geo-geometry <feature-id> <geometry-json>.");
    };
    let feature = identity
        .parse::<scriberig_core::EntityId>()
        .map_err(|_| "Feature ID must be a semantic entity ID.")?;
    let geometry = serde_json::from_str(geometry_json.trim())
        .map_err(|_| "Geometry must be valid typed protocol JSON with exact decimal strings.")?;
    let (Some(path), Some(snapshot), Some(project)) = (
        state.geo_path.clone(),
        state.geo_snapshot,
        state
            .agent_selection
            .as_ref()
            .and_then(|selection| selection.project),
    ) else {
        return Err("Load a GeoPackage catalog in the selected exact project snapshot first.");
    };
    let LoadState::Ready(catalog) = &state.geo_catalog else {
        return Err("Load the GeoPackage layer catalog before editing a feature.");
    };
    if catalog.project != project.to_string() || catalog.snapshot != snapshot.to_string() {
        return Err("The geospatial catalog no longer matches the selected project snapshot.");
    }
    let LoadState::Ready(page) = &state.geo_features else {
        return Err("Load the exact feature page before proposing a geometry edit.");
    };
    if page.project != project.to_string()
        || page.snapshot != snapshot.to_string()
        || !page
            .features
            .iter()
            .any(|row| row.id == feature.to_string())
    {
        return Err("The feature ID is not present in the loaded exact-snapshot page.");
    }
    Ok(scriberig_protocol::GeoGeometryEditRequest {
        project: project.to_string(),
        path,
        snapshot: snapshot.to_string(),
        layer: page.layer.clone(),
        feature: feature.to_string(),
        geometry,
    })
}
