//! Parses exact CAD navigation and layer-change slash commands.

use crate::{action::Action, state::TuiState};

/// Builds a bounded drawing request from the selected semantic drawing.
///
/// # Parameters
/// - `state`: Current project, selection, and exact-snapshot context.
/// - `arguments`: Optional page offset after `/cad`.
///
/// # Returns
/// Exact project, snapshot, drawing, and page offset action.
///
/// # Errors
/// Returns a concise command usage or missing-selection message.
pub(super) fn read_action(state: &TuiState, arguments: &str) -> Result<Action, &'static str> {
    let mut parts = arguments.split_whitespace();
    let offset = parts.next().map_or(Ok(0), |value| {
        value
            .parse::<usize>()
            .map_err(|_| "CAD page offset must be a non-negative integer.")
    })?;
    if parts.next().is_some() {
        return Err("Usage: /cad [offset]. Select a semantic drawing first.");
    }
    let selection = state
        .selection
        .as_ref()
        .ok_or("Select a semantic drawing at an exact snapshot first.")?;
    let agent_scope = state
        .agent_selection
        .as_ref()
        .ok_or("Select a project before inspecting a CAD drawing.")?;
    let project = agent_scope
        .project
        .ok_or("The selected context has no owning project.")?;
    let snapshot = selection.snapshot.unwrap_or(agent_scope.snapshot);
    Ok(Action::CadRequested {
        project,
        snapshot,
        drawing: selection.entity,
        offset,
    })
}

/// Builds a typed candidate request for one loaded entity and destination layer.
///
/// # Parameters
/// - `state`: Current exact CAD drawing and entity page.
/// - `arguments`: Entity and destination layer IDs.
///
/// # Returns
/// Shared protocol request anchored to the loaded snapshot.
///
/// # Errors
/// Returns usage guidance when IDs or exact drawing context are missing.
pub(super) fn layer_change(
    state: &TuiState,
    arguments: &str,
) -> Result<scriberig_protocol::CadLayerChangeRequest, &'static str> {
    let mut parts = arguments.split_whitespace();
    let entity = parse_id(parts.next(), "Usage: /cad-layer <entity-id> <layer-id>.")?;
    let destination_layer = parse_id(parts.next(), "Usage: /cad-layer <entity-id> <layer-id>.")?;
    if parts.next().is_some() {
        return Err("Usage: /cad-layer <entity-id> <layer-id>.");
    }
    let crate::inspector::LoadState::Ready(drawing) = &state.cad_drawing else {
        return Err("Load the exact CAD drawing before proposing a layer change.");
    };
    Ok(scriberig_protocol::CadLayerChangeRequest {
        project: drawing.project.clone(),
        snapshot: drawing.snapshot.clone(),
        entity: entity.to_string(),
        destination_layer: destination_layer.to_string(),
    })
}

/// Builds a semantic entity-selection action for one loaded CAD page.
///
/// # Parameters
/// - `arguments`: Entity ID following `/cad-select`.
///
/// # Returns
/// Entity selection action.
///
/// # Errors
/// Returns concise usage guidance when the ID is missing or malformed.
pub(super) fn select_entity(arguments: &str) -> Result<Action, &'static str> {
    if arguments.split_whitespace().count() != 1 {
        return Err("Usage: /cad-select <entity-id>.");
    }
    Ok(Action::CadEntitySelected {
        entity: parse_id(
            arguments.split_whitespace().next(),
            "Usage: /cad-select <entity-id>.",
        )?,
    })
}

/// Parses one semantic entity identifier without echoing malformed input.
///
/// # Parameters
/// - `value`: Optional identifier text.
/// - `usage`: Safe command usage string.
///
/// # Returns
/// Parsed semantic entity identity.
///
/// # Errors
/// Returns the provided safe usage string when parsing fails.
fn parse_id(
    value: Option<&str>,
    usage: &'static str,
) -> Result<scriberig_core::EntityId, &'static str> {
    value
        .ok_or(usage)?
        .parse::<scriberig_core::EntityId>()
        .map_err(|_| usage)
}

#[cfg(test)]
mod tests {
    use super::{layer_change, read_action};
    use crate::{
        action::Action, inspector::LoadState, selection::EntitySelection, state::TuiState,
    };

    /// Verifies `/cad` uses the selected entity and exact agent snapshot.
    #[test]
    fn drawing_command_uses_exact_selection_and_bounded_offset() {
        let project = scriberig_core::ProjectId::from_bytes([151; 16]);
        let root = scriberig_core::EntityId::from_bytes([152; 16]);
        let drawing = scriberig_core::EntityId::from_bytes([153; 16]);
        let snapshot = scriberig_core::SnapshotId::new([154; 32]);
        let state = TuiState {
            selection: Some(EntitySelection {
                entity: drawing,
                revision: None,
                snapshot: Some(snapshot),
            }),
            agent_selection: Some(scriberig_app::AgentSelection {
                project: Some(project),
                root,
                entity: drawing,
                snapshot,
            }),
            ..TuiState::default()
        };
        assert!(matches!(
            read_action(&state, "100").expect("CAD read action"),
            Action::CadRequested { project: actual_project, snapshot: actual_snapshot, drawing: actual_drawing, offset: 100 }
                if actual_project == project && actual_snapshot == snapshot && actual_drawing == drawing
        ));
    }

    /// Verifies `/cad-layer` uses the drawing's exact protocol identity.
    #[test]
    fn layer_command_uses_loaded_drawing_scope() {
        let target: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../tests/fixtures/rich-artifact/cad-entity-target.json"
        ))
        .expect("shared web/TUI CAD target fixture");
        let project: scriberig_core::ProjectId = target["project"]
            .as_str()
            .expect("project id")
            .parse()
            .expect("valid project id");
        let snapshot: scriberig_core::SnapshotId = target["snapshot"]
            .as_str()
            .expect("snapshot id")
            .parse()
            .expect("valid snapshot id");
        let entity: scriberig_core::EntityId = target["entity"]
            .as_str()
            .expect("entity id")
            .parse()
            .expect("valid entity id");
        let layer: scriberig_core::EntityId = target["destinationLayer"]
            .as_str()
            .expect("destination layer id")
            .parse()
            .expect("valid layer id");
        let state = TuiState {
            cad_drawing: LoadState::Ready(scriberig_protocol::CadDrawingInspectResponse {
                project: project.to_string(),
                snapshot: snapshot.to_string(),
                drawing: target["drawing"].as_str().expect("drawing id").to_owned(),
                units: "meters".to_owned(),
                coordinate_reference: None,
                layers: Vec::new(),
                entity_count: 0,
                complete: true,
                diagnostics: Vec::new(),
            }),
            ..TuiState::default()
        };
        let request = layer_change(&state, &format!("{entity} {layer}"))
            .expect("typed layer candidate request");
        assert_eq!(request.project, project.to_string());
        assert_eq!(request.snapshot, snapshot.to_string());
        assert_eq!(request.entity, entity.to_string());
        assert_eq!(request.destination_layer, layer.to_string());
    }
}
