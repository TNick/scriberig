//! Verifies exact-snapshot inspect and Explorer slash commands.

use super::*;

/// Opens snapshot-scoped Explorer through a local slash command.
#[test]
fn explorer_slash_command_lists_the_selected_snapshot() {
    let entity = scriberig_core::EntityId::from_bytes([157; 16]);
    let snapshot = scriberig_core::SnapshotId::new([158; 32]);
    let mut state = TuiState {
        selection: Some(crate::selection::EntitySelection {
            entity,
            revision: None,
            snapshot: Some(snapshot),
        }),
        ..TuiState::default()
    };
    state.conversation_composer.draft = "/explorer".to_owned();

    let effects = reduce(&mut state, Action::InputSubmitted);

    assert_eq!(state.primary_view, crate::selection::PrimaryView::Explorer);
    assert!(matches!(effects.as_slice(), [Effect::ListExplorer {
        entity: actual_entity,
        snapshot: actual_snapshot,
        continuation: None,
        ..
    }] if *actual_entity == entity && *actual_snapshot == snapshot));
    reduce(
        &mut state,
        Action::ViewSelected(crate::selection::PrimaryView::Conversation),
    );
    assert_eq!(
        state.primary_view,
        crate::selection::PrimaryView::Conversation
    );
}

/// Inspects one evidence entity at the exact snapshot shown by a plan view.
#[test]
fn inspect_slash_command_accepts_an_exact_snapshot() {
    let entity = scriberig_core::EntityId::from_bytes([163; 16]);
    let snapshot = scriberig_core::SnapshotId::new([164; 32]);
    let mut state = TuiState::default();
    state.conversation_composer.draft = format!("/inspect {entity} --snapshot {snapshot}");

    let effects = reduce(&mut state, Action::InputSubmitted);

    assert!(matches!(effects.as_slice(), [Effect::InspectEntity {
        entity: actual_entity,
        snapshot: Some(actual_snapshot),
        ..
    }] if *actual_entity == entity && *actual_snapshot == snapshot));
}
