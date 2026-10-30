//! Verifies bounded artifact effects use the shared TUI service contract.

use super::*;

/// Runs artifact page effects through the application-backed TUI service adapter.
#[test]
fn artifact_page_effect_uses_the_shared_query_service() {
    let service = ApplicationAdapter::new(Fixture, Vec::new());
    let project = scriberig_core::ProjectId::from_bytes([101; 16]);
    let action = execute_effect(
        &service,
        &ApplicationContext::empty(),
        Effect::InspectArtifactPage {
            request_id: 7,
            project,
            offset: 20,
            limit: 20,
        },
    );
    assert!(matches!(
        action,
        Action::ArtifactsLoaded { request_id: 7, model }
            if model.project == project.to_string() && model.artifacts.is_empty()
    ));
}

/// Routes exact artifact-member effects through shared application query semantics.
#[test]
fn artifact_member_effect_uses_the_shared_query_service() {
    let service = ApplicationAdapter::new(Fixture, Vec::new());
    let project = scriberig_core::ProjectId::from_bytes([101; 16]);
    let artifact = scriberig_core::EntityId::from_bytes([102; 16]);
    let action = execute_effect(
        &service,
        &ApplicationContext::empty(),
        Effect::InspectArtifactMembers {
            request_id: 8,
            project,
            artifact,
            offset: 20,
            limit: 20,
        },
    );
    assert!(matches!(
        action,
        Action::ArtifactMembersLoaded { request_id: 8, model }
            if model.project == project.to_string()
                && model.artifact == artifact.to_string()
                && model.members.is_empty()
    ));
}
