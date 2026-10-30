//! Verifies bounded CAD navigation and exact-base layer candidate transitions.

use super::super::*;
use crate::{action::Action, effect::Effect, inspector::LoadState, selection::PrimaryView};

/// Returns the canonical snapshot identity shared by reducer fixtures.
///
/// # Returns
/// Stable test snapshot text accepted by the production ID parser.
fn snapshot_text() -> String {
    scriberig_core::SnapshotId::new([2; 32]).to_string()
}

/// Returns the canonical project identity shared by CAD reducer fixtures.
///
/// # Returns
/// Stable project text accepted by the production ID parser.
fn project_text() -> String {
    scriberig_core::ProjectId::from_bytes([1; 16]).to_string()
}

/// Returns the canonical drawing identity shared by CAD reducer fixtures.
///
/// # Returns
/// Stable drawing text accepted by the production entity ID parser.
fn drawing_text() -> String {
    scriberig_core::EntityId::from_bytes([3; 16]).to_string()
}

/// Returns the canonical entity identity shared by CAD reducer fixtures.
///
/// # Returns
/// Stable entity text accepted by the production entity ID parser.
fn entity_text() -> String {
    scriberig_core::EntityId::from_bytes([6; 16]).to_string()
}

/// Builds a complete drawing and bounded page fixture for reducer tests.
///
/// # Returns
/// TUI state with two layers and one semantic line entity loaded.
fn loaded_state() -> TuiState {
    TuiState {
        cad_drawing: LoadState::Ready(scriberig_protocol::CadDrawingInspectResponse {
            project: project_text(),
            snapshot: snapshot_text(),
            drawing: drawing_text(),
            units: "meters".to_owned(),
            coordinate_reference: None,
            layers: vec![
                scriberig_protocol::CadLayerSummary {
                    id: "layer-4".to_owned(),
                    name: "PARCELS".to_owned(),
                    visible: true,
                    locked: false,
                },
                scriberig_protocol::CadLayerSummary {
                    id: "layer-5".to_owned(),
                    name: "LABELS".to_owned(),
                    visible: true,
                    locked: false,
                },
            ],
            entity_count: 1,
            complete: true,
            diagnostics: Vec::new(),
        }),
        cad_entities: LoadState::Ready(scriberig_protocol::CadEntitiesResponse {
            project: project_text(),
            snapshot: snapshot_text(),
            drawing: drawing_text(),
            entities: vec![scriberig_protocol::CadEntitySummary {
                id: entity_text(),
                layer: "layer-4".to_owned(),
                space: scriberig_protocol::CadDrawingSpace::Model,
                geometry: scriberig_protocol::CadGeometry::Text {
                    position: scriberig_protocol::CadCoordinate {
                        x: "2".to_owned(),
                        y: "3".to_owned(),
                        z: "0".to_owned(),
                    },
                    value: "Parcel 124".to_owned(),
                    height: "0.25".to_owned(),
                },
                style: None,
            }],
            total: 1,
            has_more: false,
        }),
        ..TuiState::default()
    }
}

/// Verifies a CAD read requests a bounded page at the exact supplied scope.
#[test]
fn cad_read_requests_exact_drawing_and_bounded_entity_page() {
    let mut state = TuiState::default();
    let project = scriberig_core::ProjectId::from_bytes([1; 16]);
    let snapshot = scriberig_core::SnapshotId::new([2; 32]);
    let drawing = scriberig_core::EntityId::from_bytes([3; 16]);
    let effects = crate::state::reduce(
        &mut state,
        Action::CadRequested {
            project,
            snapshot,
            drawing,
            offset: 100,
        },
    );
    assert!(matches!(
        effects.as_slice(),
        [Effect::InspectCadPage { drawing: summary, entities, .. }]
            if summary.snapshot == snapshot_text()
                && summary.drawing == drawing.to_string()
                && entities.offset == 100
                && entities.limit == 100
    ));
    assert!(matches!(state.cad_drawing, LoadState::Loading { .. }));
}

/// Verifies a candidate request is scoped to an entity and unlocked layer in the loaded page.
#[test]
fn cad_layer_candidate_requires_loaded_target_and_opens_shared_review() {
    let mut state = loaded_state();
    let request = scriberig_protocol::CadLayerChangeRequest {
        project: project_text(),
        snapshot: snapshot_text(),
        entity: entity_text(),
        destination_layer: "layer-5".to_owned(),
    };
    let effects = crate::state::reduce(&mut state, Action::CadLayerChangeRequested(request));
    let LoadState::Loading { request_id } = state.cad_layer_edit else {
        panic!("candidate request must be loading");
    };
    assert!(matches!(
        effects.as_slice(),
        [Effect::ProposeCadLayerChange { .. }]
    ));

    let candidate = scriberig_protocol::CadLayerChangeResponse {
        candidate: "candidate-7".to_owned(),
        project: project_text(),
        base_snapshot: snapshot_text(),
        path: "drawing.dxf".to_owned(),
        validation: scriberig_protocol::CadCandidateValidation::Pending,
    };
    let effects = crate::state::reduce(
        &mut state,
        Action::CadLayerChangeProposed {
            request_id,
            candidate,
        },
    );
    assert_eq!(state.primary_view, PrimaryView::Changes);
    assert!(matches!(state.cad_layer_edit, LoadState::Ready(_)));
    assert_ne!(effects.len(), 0);
}

/// Verifies a destination layer outside the drawing cannot produce a candidate.
#[test]
fn cad_layer_candidate_rejects_unknown_destination() {
    let mut state = loaded_state();
    let effects = crate::state::reduce(
        &mut state,
        Action::CadLayerChangeRequested(scriberig_protocol::CadLayerChangeRequest {
            project: project_text(),
            snapshot: snapshot_text(),
            entity: entity_text(),
            destination_layer: "foreign-layer".to_owned(),
        }),
    );
    assert_eq!(effects, vec![Effect::None]);
    assert!(state.conversation_composer.error.is_some());
}

/// Verifies entity selection retains the loaded exact drawing snapshot.
#[test]
fn cad_entity_selection_keeps_exact_snapshot_identity() {
    let mut state = loaded_state();
    let entity = scriberig_core::EntityId::from_bytes([6; 16]);
    crate::state::reduce(&mut state, Action::CadEntitySelected { entity });
    let selected = state.selection.expect("selected entity");
    assert_eq!(selected.entity, entity);
    assert_eq!(selected.snapshot.unwrap().to_string(), snapshot_text());
    let agent_selection = state.agent_selection.expect("selected agent scope");
    assert_eq!(agent_selection.entity, entity);
    assert_eq!(agent_selection.snapshot.to_string(), snapshot_text());
}
