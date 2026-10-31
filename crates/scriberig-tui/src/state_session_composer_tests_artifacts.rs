//! Verifies artifact inspection slash-command request behavior.

use super::*;

/// Verifies artifact slash commands request exact project pages with bounded limits.
#[test]
fn artifacts_command_requests_a_bounded_project_page() {
    let mut state = TuiState::default();
    let project_text = format!("prj_{}", "07".repeat(16));
    state.conversation_composer.draft = format!("/artifacts {project_text} 20");
    let effects = reduce(&mut state, Action::InputSubmitted);
    let project = project_text.parse().expect("project id");
    assert!(matches!(
        effects.as_slice(),
        [Effect::InspectArtifactPage {
            project: requested,
            offset: 20,
            limit: 20,
            ..
        }] if *requested == project
    ));
    assert!(matches!(
        state.artifact_page,
        crate::inspector::LoadState::Loading { .. }
    ));
}

/// Verifies artifact member commands carry exact artifact identity and page offset.
#[test]
fn artifact_members_command_requests_a_bounded_page() {
    let project = scriberig_core::ProjectId::from_bytes([17; 16]);
    let artifact = scriberig_core::EntityId::from_bytes([18; 16]);
    let mut state = TuiState {
        agent_selection: Some(scriberig_app::AgentSelection {
            project: Some(project),
            root: artifact,
            entity: artifact,
            snapshot: scriberig_core::SnapshotId::new([19; 32]),
        }),
        ..TuiState::default()
    };
    state.conversation_composer.draft = format!("/artifact-members {project} {artifact} 40");
    let effects = reduce(&mut state, Action::InputSubmitted);
    assert!(matches!(
        effects.as_slice(),
        [Effect::InspectArtifactMembers {
            project: requested_project,
            artifact: requested_artifact,
            offset: 40,
            limit: 20,
            ..
        }] if *requested_project == project && *requested_artifact == artifact
    ));
    assert!(matches!(
        state.artifact_members,
        crate::inspector::LoadState::Loading { .. }
    ));
}
