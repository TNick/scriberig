//! Parses terminal geospatial commands against the selected exact project snapshot.

use crate::{inspector::LoadState, state::TuiState};

#[path = "geo_command/geometry.rs"]
mod geometry;
#[path = "geo_command/query.rs"]
mod query;
pub(super) use geometry::geometry_edit;
pub(super) use query::query_request;

/// Resolves the current project and exact snapshot for geospatial reads.
///
/// # Parameters
/// - `state`: Current TUI selection and active project scope.
///
/// # Returns
/// Stable project and snapshot identities, or a safe user-facing error.
///
/// # Errors
/// Returns a user-facing error when no project or exact snapshot is selected.
pub(super) fn scope(
    state: &TuiState,
) -> Result<(scriberig_core::ProjectId, scriberig_core::SnapshotId), &'static str> {
    let project = state
        .agent_selection
        .as_ref()
        .and_then(|selection| selection.project)
        .ok_or("Geospatial inspection requires an active project.")?;
    let snapshot = state
        .selection
        .and_then(|selection| selection.snapshot)
        .ok_or("Select an exact snapshot before geospatial inspection.")?;
    Ok((project, snapshot))
}

/// Parses a project-relative source path and optional catalog page offset.
///
/// # Parameters
/// - `arguments`: Text following `/geo`.
///
/// # Returns
/// Exact path and zero-based offset.
///
/// # Errors
/// Returns a usage message for missing path or invalid offset.
pub(super) fn catalog_arguments(arguments: &str) -> Result<(String, usize), &'static str> {
    let mut parts = arguments.split_whitespace();
    let path = parts
        .next()
        .ok_or("Usage: /geo <project-relative-path> [offset].")?;
    if path.starts_with('/') || path.split('/').any(|part| part == "..") {
        return Err("GeoPackage path must be project-relative and cannot traverse parents.");
    }
    let offset = parts
        .next()
        .map(|value| {
            value
                .parse()
                .map_err(|_| "GeoPackage offset must be a non-negative integer.")
        })
        .transpose()?
        .unwrap_or(0);
    if parts.next().is_some() {
        return Err("Usage: /geo <project-relative-path> [offset].");
    }
    Ok((path.to_owned(), offset))
}

/// Parses a layer ID and optional feature page offset.
///
/// # Parameters
/// - `arguments`: Text following `/geo-features`.
///
/// # Returns
/// Stable semantic layer identity and zero-based offset.
///
/// # Errors
/// Returns a usage message for an invalid identity or offset.
pub(super) fn feature_arguments(
    arguments: &str,
) -> Result<(scriberig_core::EntityId, usize), &'static str> {
    let mut parts = arguments.split_whitespace();
    let layer = parts
        .next()
        .ok_or("Usage: /geo-features <layer-id> [offset].")?
        .parse()
        .map_err(|_| "Layer ID must be a semantic entity ID.")?;
    let offset = parts
        .next()
        .map(|value| {
            value
                .parse()
                .map_err(|_| "Feature offset must be a non-negative integer.")
        })
        .transpose()?
        .unwrap_or(0);
    if parts.next().is_some() {
        return Err("Usage: /geo-features <layer-id> [offset].");
    }
    Ok((layer, offset))
}

/// Builds a schema-checked exact-base attribute candidate request from loaded geospatial data.
///
/// # Parameters
/// - `state`: TUI state with a loaded exact-snapshot feature page and catalog.
/// - `arguments`: Feature ID, field name, and typed `kind:value` replacement.
///
/// # Returns
/// Shared project candidate request for one verified feature and field.
///
/// # Errors
/// Returns a safe usage or current-snapshot error when arguments do not match loaded data.
pub(super) fn attribute_edit(
    state: &TuiState,
    arguments: &str,
) -> Result<scriberig_project::GeoPackageFeatureAttributeEdit, &'static str> {
    let Some((identity, typed_value)) = arguments.split_once(char::is_whitespace) else {
        return Err("Usage: /geo-set <feature-id> <field> <kind:value>.");
    };
    let feature = identity
        .parse::<scriberig_core::EntityId>()
        .map_err(|_| "Feature ID must be a semantic entity ID.")?;
    let Some((field, raw_value)) = typed_value.trim().split_once(char::is_whitespace) else {
        return Err("Usage: /geo-set <feature-id> <field> <kind:value>.");
    };
    let typed = raw_value.trim();
    let (kind, value) = if typed == "null" {
        ("null", "")
    } else if let Some((kind, value)) = typed.split_once(':') {
        (kind, value)
    } else {
        return Err(
            "Use a typed value: text:<value>, number:<value>, boolean:true|false, datetime:<value>, or null.",
        );
    };
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
        return Err("Load the exact feature page before proposing an attribute edit.");
    };
    if page.snapshot != snapshot.to_string()
        || !page
            .features
            .iter()
            .any(|row| row.id == feature.to_string())
    {
        return Err("The feature ID is not present in the loaded exact-snapshot page.");
    }
    let layer_id = page.layer.clone();
    let layer = catalog
        .layers
        .iter()
        .find(|layer| layer.id == layer_id)
        .ok_or("The feature layer is not present in the loaded catalog page.")?;
    let schema = layer
        .fields
        .iter()
        .find(|item| item.name == field)
        .ok_or("The field is not present in the loaded layer schema.")?;
    let value = match (kind, value) {
        ("text", value) => scriberig_domain_geo::GeoValue::Text(value.to_owned()),
        ("number", value) if !value.is_empty() => {
            scriberig_domain_geo::GeoValue::Number(value.to_owned())
        }
        ("boolean", "true") => scriberig_domain_geo::GeoValue::Boolean(true),
        ("boolean", "false") => scriberig_domain_geo::GeoValue::Boolean(false),
        ("datetime", value) => scriberig_domain_geo::GeoValue::DateTime(value.to_owned()),
        ("null", "") => scriberig_domain_geo::GeoValue::Null,
        _ => return Err("Typed value is invalid; use text, number, boolean, datetime, or null."),
    };
    let type_matches = matches!(
        (&schema.field_type, &value),
        (
            scriberig_protocol::GeoFieldType::Text,
            scriberig_domain_geo::GeoValue::Text(_)
        ) | (
            scriberig_protocol::GeoFieldType::Number,
            scriberig_domain_geo::GeoValue::Number(_)
        ) | (
            scriberig_protocol::GeoFieldType::Boolean,
            scriberig_domain_geo::GeoValue::Boolean(_)
        ) | (
            scriberig_protocol::GeoFieldType::DateTime,
            scriberig_domain_geo::GeoValue::DateTime(_)
        ) | (_, scriberig_domain_geo::GeoValue::Null)
    );
    if !type_matches || (matches!(value, scriberig_domain_geo::GeoValue::Null) && !schema.nullable)
    {
        return Err("Replacement value does not match the field type or nullability.");
    }
    Ok(scriberig_project::GeoPackageFeatureAttributeEdit {
        project,
        base_snapshot: snapshot,
        path,
        layer: layer
            .id
            .parse()
            .map_err(|_| "Loaded semantic layer ID is invalid.")?,
        feature,
        field: field.to_owned(),
        value,
    })
}

#[cfg(test)]
#[path = "geo_command/tests.rs"]
mod tests;
