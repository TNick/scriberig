//! Parses QGIS project inspection and linked-layer navigation commands.

use crate::{action::Action, inspector::LoadState, state::TuiState};

/// Creates an exact-snapshot request for a project-relative QGIS file.
///
/// # Parameters
/// - `state`: Current active project and exact snapshot selection.
/// - `arguments`: Text following `/qgis`.
///
/// # Returns
/// QGIS project inspection action.
///
/// # Errors
/// Returns a safe usage or scope error for invalid input.
pub(super) fn project_request(state: &TuiState, arguments: &str) -> Result<Action, &'static str> {
    let (project, snapshot) = super::geo_command::scope(state)?;
    let mut parts = arguments.split_whitespace();
    let path = parts
        .next()
        .ok_or("Usage: /qgis <project-relative-path>.")?;
    if path.starts_with('/') || path.split('/').any(|part| part == "..") {
        return Err("QGIS project path must be project-relative and cannot traverse parents.");
    }
    if parts.next().is_some() || !matches!(PathSuffix::classify(path), Some("qgs" | "qgz")) {
        return Err("Usage: /qgis <project-relative-path.qgs|.qgz>.");
    }
    Ok(Action::QgisProjectRequested(
        scriberig_app::QgisProjectQuery {
            project,
            snapshot,
            path: path.to_owned(),
        },
    ))
}

/// Creates a catalog request for a loaded and resolved QGIS layer reference.
///
/// # Parameters
/// - `state`: Current TUI state containing the loaded QGIS project.
/// - `arguments`: Text following `/qgis-layer`.
///
/// # Returns
/// Exact-snapshot `GeoPackage` catalog request for the linked layer source.
///
/// # Errors
/// Returns an error when the reference is unresolved, stale, or unknown.
pub(super) fn layer_request(state: &TuiState, arguments: &str) -> Result<Action, &'static str> {
    let mut parts = arguments.split_whitespace();
    let reference = parts.next().ok_or("Usage: /qgis-layer <reference-id>.")?;
    let reference = reference
        .parse::<scriberig_core::EntityId>()
        .map_err(|_| "QGIS reference ID must be a semantic entity ID.")?;
    if parts.next().is_some() {
        return Err("Usage: /qgis-layer <reference-id>.");
    }
    let LoadState::Ready(project_view) = &state.qgis_project else {
        return Err("Load a QGIS project first with `/qgis <path>`.");
    };
    let layer = project_view
        .layers
        .iter()
        .find(|layer| layer.id == reference.to_string())
        .ok_or("That reference is not present in the loaded QGIS project.")?;
    let (Some(path), Some(_layer)) = (&layer.resolved_path, &layer.layer) else {
        return Err(
            "That QGIS reference has no resolved GeoLayer. Check its resolution diagnostic.",
        );
    };
    Ok(Action::GeoCatalogRequested {
        project: project_view
            .project
            .parse()
            .map_err(|_| "Loaded QGIS project identity is invalid.")?,
        path: path.clone(),
        snapshot: project_view
            .snapshot
            .parse()
            .map_err(|_| "Loaded QGIS snapshot identity is invalid.")?,
        offset: 0,
    })
}

/// Creates a rename candidate request for a reference in the loaded exact QGIS view.
///
/// # Parameters
/// - `state`: TUI state containing the active scope and loaded QGIS project.
/// - `arguments`: Reference ID followed by the new display name.
///
/// # Returns
/// Exact-base QGIS layer rename action.
///
/// # Errors
/// Returns an error when the loaded view is stale, the reference is absent, or the name is invalid.
pub(super) fn rename_request(state: &TuiState, arguments: &str) -> Result<Action, &'static str> {
    let (reference, new_name) = arguments
        .split_once(char::is_whitespace)
        .ok_or("Usage: /qgis-rename <reference-id> <new-name>.")?;
    let reference = reference
        .parse::<scriberig_core::EntityId>()
        .map_err(|_| "QGIS reference ID must be a semantic entity ID.")?;
    let new_name = new_name.trim();
    if new_name.is_empty() {
        return Err("A QGIS layer display name cannot be empty.");
    }
    let LoadState::Ready(project_view) = &state.qgis_project else {
        return Err("Load a QGIS project first with `/qgis <path>`.");
    };
    let active_project = state
        .agent_selection
        .as_ref()
        .and_then(|selection| selection.project)
        .ok_or("QGIS rename requires an active project.")?;
    let active_snapshot = state
        .selection
        .and_then(|selection| selection.snapshot)
        .ok_or("QGIS rename requires an exact selected snapshot.")?;
    if project_view.project != active_project.to_string()
        || project_view.snapshot != active_snapshot.to_string()
    {
        return Err("Reload the QGIS project at the active exact snapshot before renaming.");
    }
    if !project_view
        .layers
        .iter()
        .any(|layer| layer.id == reference.to_string())
    {
        return Err("That reference is not present in the loaded QGIS project.");
    }
    Ok(Action::QgisLayerRenameRequested(
        scriberig_protocol::QgisLayerRenameRequest {
            project: project_view.project.clone(),
            snapshot: project_view.snapshot.clone(),
            reference: reference.to_string(),
            new_name: new_name.to_owned(),
        },
    ))
}

/// Creates a full layer-order candidate request from semantic reference IDs.
///
/// # Parameters
/// - `state`: TUI state containing the active scope and loaded QGIS project.
/// - `arguments`: Ordered whitespace-separated reference IDs.
///
/// # Returns
/// Exact-snapshot QGIS layer-order action.
///
/// # Errors
/// Returns an error when the loaded project is stale or IDs are not a permutation.
pub(super) fn reorder_request(state: &TuiState, arguments: &str) -> Result<Action, &'static str> {
    let LoadState::Ready(project_view) = &state.qgis_project else {
        return Err("Load a QGIS project first with `/qgis <path>`.");
    };
    let active_project = state
        .agent_selection
        .as_ref()
        .and_then(|selection| selection.project)
        .ok_or("QGIS layer order requires an active project.")?;
    let active_snapshot = state
        .selection
        .and_then(|selection| selection.snapshot)
        .ok_or("QGIS layer order requires an exact selected snapshot.")?;
    if project_view.project != active_project.to_string()
        || project_view.snapshot != active_snapshot.to_string()
    {
        return Err("Reload the QGIS project at the active exact snapshot before reordering.");
    }
    if project_view.groups.len() != 1
        || project_view.groups[0].layers.len() != project_view.layers.len()
    {
        return Err("QGIS layer order currently requires one flat project tree.");
    }
    let ordered_references = arguments
        .split_whitespace()
        .map(|value| {
            value
                .parse::<scriberig_core::EntityId>()
                .map(|id| id.to_string())
                .map_err(|_| "QGIS reference IDs must be semantic entity IDs.")
        })
        .collect::<Result<Vec<_>, _>>()?;
    let known = project_view
        .layers
        .iter()
        .map(|layer| layer.id.as_str())
        .collect::<std::collections::HashSet<_>>();
    let requested = ordered_references
        .iter()
        .map(String::as_str)
        .collect::<std::collections::HashSet<_>>();
    if ordered_references.is_empty()
        || known.len() != project_view.layers.len()
        || requested.len() != ordered_references.len()
        || known != requested
    {
        return Err("Provide every loaded QGIS reference exactly once in the desired order.");
    }
    if project_view
        .layers
        .iter()
        .map(|layer| layer.id.as_str())
        .eq(ordered_references.iter().map(String::as_str))
    {
        return Err("The requested QGIS layer order is already active.");
    }
    Ok(Action::QgisLayerOrderRequested(
        scriberig_protocol::QgisLayerOrderRequest {
            project: project_view.project.clone(),
            snapshot: project_view.snapshot.clone(),
            path: project_view.path.clone(),
            ordered_references,
        },
    ))
}

/// Classifies a project path suffix without allocating another path buffer.
struct PathSuffix;

impl PathSuffix {
    /// Returns a lowercase accepted suffix for a project filename.
    ///
    /// # Parameters
    /// - `path`: Project-relative path.
    ///
    /// # Returns
    /// `qgs` or `qgz` for supported project files, otherwise `None`.
    fn classify(path: &str) -> Option<&str> {
        match path.rsplit_once('.')?.1 {
            suffix if suffix.eq_ignore_ascii_case("qgs") => Some("qgs"),
            suffix if suffix.eq_ignore_ascii_case("qgz") => Some("qgz"),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{layer_request, project_request, rename_request, reorder_request};
    use crate::{action::Action, inspector::LoadState, state::TuiState};
    use scriberig_core::{EntityId, ProjectId, SnapshotId};

    /// Creates a TUI state bound to one exact project snapshot.
    ///
    /// # Returns
    /// A state initialized with one active project, snapshot, and QGIS reference.
    fn state() -> (TuiState, ProjectId, SnapshotId, EntityId) {
        let project = ProjectId::from_bytes([1; 16]);
        let snapshot = SnapshotId::new([2; 32]);
        let root = EntityId::from_bytes([3; 16]);
        let reference = EntityId::from_bytes([4; 16]);
        let state = TuiState {
            agent_selection: Some(scriberig_app::AgentSelection {
                project: Some(project),
                root,
                entity: root,
                snapshot,
            }),
            selection: Some(crate::selection::EntitySelection {
                entity: root,
                revision: None,
                snapshot: Some(snapshot),
            }),
            qgis_project: LoadState::Ready(scriberig_protocol::QgisProjectResponse {
                project: project.to_string(),
                path: "project.qgs".to_owned(),
                snapshot: snapshot.to_string(),
                revision: "revision-1".to_owned(),
                id: "qgis-project-1".to_owned(),
                title: Some("Planning".to_owned()),
                crs: None,
                layers: vec![
                    scriberig_protocol::QgisLayerReferenceView {
                        id: reference.to_string(),
                        name: "Parcels".to_owned(),
                        data_source: "./parcels.gpkg".to_owned(),
                        path: Some("./parcels.gpkg".to_owned()),
                        resolved_path: Some("parcels.gpkg".to_owned()),
                        source_layer: Some("parcels".to_owned()),
                        dataset: Some("dataset-1".to_owned()),
                        layer: Some("layer-1".to_owned()),
                        resolution_diagnostic: None,
                        visible: true,
                        renderer: None,
                        scale_range: None,
                    },
                    scriberig_protocol::QgisLayerReferenceView {
                        id: scriberig_core::EntityId::from_bytes([5; 16]).to_string(),
                        name: "Roads".to_owned(),
                        data_source: "roads.gpkg".to_owned(),
                        path: Some("roads.gpkg".to_owned()),
                        resolved_path: Some("roads.gpkg".to_owned()),
                        source_layer: Some("roads".to_owned()),
                        dataset: None,
                        layer: None,
                        resolution_diagnostic: None,
                        visible: true,
                        renderer: None,
                        scale_range: None,
                    },
                ],
                groups: vec![scriberig_protocol::QgisLayerGroupView {
                    id: "group-1".to_owned(),
                    name: "root".to_owned(),
                    layers: vec![
                        reference.to_string(),
                        scriberig_core::EntityId::from_bytes([5; 16]).to_string(),
                    ],
                }],
            }),
            ..TuiState::default()
        };
        (state, project, snapshot, reference)
    }

    /// Keeps QGIS project reads bound to the current exact project selection.
    #[test]
    fn project_request_keeps_exact_scope_and_rejects_escape() {
        let (state, project, snapshot, _) = state();
        assert_eq!(
            project_request(&state, "project.qgs").expect("project request"),
            Action::QgisProjectRequested(scriberig_app::QgisProjectQuery {
                project,
                snapshot,
                path: "project.qgs".to_owned(),
            })
        );
        assert!(project_request(&state, "../project.qgs").is_err());
    }

    /// Requires a complete identity permutation when building an order action.
    #[test]
    fn reorder_request_uses_exact_loaded_reference_order() {
        let (state, project, snapshot, reference) = state();
        let second = EntityId::from_bytes([5; 16]);
        let action =
            reorder_request(&state, &format!("{second} {reference}")).expect("order request");
        assert_eq!(
            action,
            Action::QgisLayerOrderRequested(scriberig_protocol::QgisLayerOrderRequest {
                project: project.to_string(),
                snapshot: snapshot.to_string(),
                path: "project.qgs".to_owned(),
                ordered_references: vec![second.to_string(), reference.to_string()],
            })
        );
        assert!(reorder_request(&state, "").is_err());
        assert!(reorder_request(&state, &format!("{reference} {reference}")).is_err());
    }

    /// Routes resolved QGIS references into their exact linked `GeoPackage` path.
    #[test]
    fn layer_request_opens_linked_geopackage_catalog() {
        let (state, project, snapshot, reference) = state();
        assert_eq!(
            layer_request(&state, &reference.to_string()).expect("linked source"),
            Action::GeoCatalogRequested {
                project,
                path: "parcels.gpkg".to_owned(),
                snapshot,
                offset: 0,
            }
        );
    }

    /// Builds a rename request only for a loaded reference and its exact active snapshot.
    #[test]
    fn rename_request_keeps_loaded_project_and_snapshot_scope() {
        let (state, project, snapshot, reference) = state();
        assert_eq!(
            rename_request(&state, &format!("{reference} Parcels Updated"))
                .expect("rename request"),
            Action::QgisLayerRenameRequested(scriberig_protocol::QgisLayerRenameRequest {
                project: project.to_string(),
                snapshot: snapshot.to_string(),
                reference: reference.to_string(),
                new_name: "Parcels Updated".to_owned(),
            })
        );
        assert!(rename_request(&state, "unknown Name").is_err());
    }
}
