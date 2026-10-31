//! Renders exact-snapshot QGIS project references and linked-layer navigation.

use super::super::common::{append_value, render_load_state};
use crate::inspector::LoadState;
use scriberig_protocol::QgisProjectResponse;

/// Renders QGIS project structure and commands for linked `GeoPackage` layers.
///
/// # Parameters
/// - `state`: Exact-snapshot QGIS project load state.
///
/// # Returns
/// Bounded project and layer-reference details.
pub(super) fn render(state: &LoadState<QgisProjectResponse>) -> String {
    render_load_state("QGIS project", state, |project, output| {
        append_value(
            output,
            &format!(
                "Project: {}",
                project.title.as_deref().unwrap_or("(untitled)")
            ),
        );
        append_value(output, &format!("Path: {}", project.path));
        append_value(output, &format!("Snapshot: {}", project.snapshot));
        append_value(output, &format!("Revision: {}", project.revision));
        append_value(
            output,
            "Rename a reference with `/qgis-rename <reference-id> <new-name>`.",
        );
        append_value(
            output,
            "Set a flat project order with `/qgis-order <reference-id> ...`.",
        );
        for layer in &project.layers {
            append_value(output, &format!("## {} · {}", layer.name, layer.id));
            append_value(output, &format!("Source: {}", layer.data_source));
            append_value(output, &format!("Visible: {}", layer.visible));
            if let Some(diagnostic) = &layer.resolution_diagnostic {
                append_value(output, &format!("Resolution: {diagnostic}"));
            } else if layer.layer.is_some() {
                append_value(
                    output,
                    &format!("GeoLayer: {}", layer.layer.as_deref().unwrap_or_default()),
                );
                append_value(output, &format!("Open source: `/qgis-layer {}`", layer.id));
            }
        }
        for group in &project.groups {
            append_value(
                output,
                &format!("Group {} · {}", group.name, group.layers.join(", ")),
            );
        }
    })
}

#[cfg(test)]
mod tests {
    use super::render;
    use crate::inspector::LoadState;
    use scriberig_protocol::{QgisLayerReferenceView, QgisProjectResponse};

    /// Keeps provider resolution diagnostics visible for unresolved QGIS references.
    #[test]
    fn unresolved_reference_diagnostic_is_rendered() {
        let view = QgisProjectResponse {
            project: "project-1".to_owned(),
            path: "maps/plan.qgs".to_owned(),
            snapshot: "snapshot-1".to_owned(),
            revision: "revision-1".to_owned(),
            id: "qgis-project-1".to_owned(),
            title: Some("Plan".to_owned()),
            crs: None,
            layers: vec![QgisLayerReferenceView {
                id: "reference-1".to_owned(),
                name: "Parcels".to_owned(),
                data_source: "missing.gpkg".to_owned(),
                path: Some("missing.gpkg".to_owned()),
                resolved_path: None,
                source_layer: Some("parcels".to_owned()),
                dataset: None,
                layer: None,
                resolution_diagnostic: Some("source artifact is absent".to_owned()),
                visible: true,
                renderer: None,
                scale_range: None,
            }],
            groups: Vec::new(),
        };
        let output = render(&LoadState::Ready(view));
        assert!(output.contains("Resolution: source artifact is absent"));
        assert!(output.contains("reference-1"));
    }
}
