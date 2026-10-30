//! Verifies TUI service control effect mapping.

use super::*;

/// Verifies control effects are translated into explicit reducer actions.
#[test]
fn translates_control_effects_without_silent_noops() {
    let service = ApplicationAdapter::new(Fixture, Vec::new());
    let context = ApplicationContext::empty();
    assert_eq!(
        execute_effect(&service, &context, Effect::RequestQuit),
        Action::QuitRequested
    );
    assert_eq!(
        execute_effect(&service, &context, Effect::RequestRefresh),
        Action::RefreshRequested
    );
    assert!(matches!(
        execute_effect(
            &service,
            &context,
            Effect::RequestFind {
                request_id: 1,
                text: "term".to_owned(),
                snapshot: SnapshotId::new([8; 32]),
            }
        ),
        Action::OperationFailed { category, .. } if category == "unsupported_operation"
    ));
}
