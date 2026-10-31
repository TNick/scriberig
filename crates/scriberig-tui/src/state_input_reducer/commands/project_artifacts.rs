//! Parses bounded semantic project-artifact listing commands.

use super::*;

/// Reports the currently active project without requesting artifact data.
///
/// - `state`: Current reducer state.
///
/// # Returns
/// A presentation-only status effect.
pub(super) fn active_project(state: &mut TuiState) -> Vec<Effect> {
    state.shell_message = state.project.as_ref().map_or_else(
        || "No active project is selected.".to_owned(),
        |name| format!("Active project: {name}"),
    );
    vec![Effect::None]
}

/// Lists a bounded project artifact page.
///
/// # Parameters
/// - `state`: Current reducer state.
/// - `command`: Full artifact-list command.
///
/// # Returns
/// The resulting effect, or no result when the command is unhandled.
pub(super) fn artifacts(state: &mut TuiState, command: &str) -> Vec<Effect> {
    let mut arguments = command.split_whitespace().skip(1);
    let project = match arguments.next() {
        Some(value) => {
            if let Ok(project) = value.parse::<scriberig_core::ProjectId>() {
                Some(project)
            } else {
                state.conversation_composer.error =
                    Some("Artifact inspection expects a project id such as prj_<hex>.".to_owned());
                return vec![Effect::None];
            }
        }
        None => state
            .agent_selection
            .as_ref()
            .and_then(|selection| selection.project),
    };
    let Some(project) = project else {
        state.conversation_composer.error =
            Some("Supply a project id: /artifacts prj_<hex> [offset].".to_owned());
        return vec![Effect::None];
    };
    let offset = match arguments.next() {
        Some(value) => {
            if let Ok(offset) = value.parse::<usize>() {
                offset
            } else {
                state.conversation_composer.error =
                    Some("Artifact offset must be a non-negative integer.".to_owned());
                return vec![Effect::None];
            }
        }
        None => 0,
    };
    if arguments.next().is_some() {
        state.conversation_composer.error =
            Some("Usage: /artifacts [project-id] [offset].".to_owned());
        return vec![Effect::None];
    }
    super::reduce(state, Action::ArtifactsRequested { project, offset })
}

/// Lists a bounded artifact-member page.
///
/// # Parameters
/// - `state`: Current reducer state.
/// - `command`: Full artifact-member command.
///
/// # Returns
/// The resulting effect, or no result when the command is unhandled.
pub(super) fn members(state: &mut TuiState, command: &str) -> Vec<Effect> {
    let mut arguments = command.split_whitespace().skip(1);
    let (project, artifact) =
        if let (Some(project), Some(artifact)) = (arguments.next(), arguments.next()) {
            if let (Ok(project), Ok(artifact)) = (project.parse(), artifact.parse()) {
                (project, artifact)
            } else {
                state.conversation_composer.error =
                    Some("Usage: /artifact-members prj_<hex> ent_<hex> [offset].".to_owned());
                return vec![Effect::None];
            }
        } else {
            state.conversation_composer.error =
                Some("Usage: /artifact-members prj_<hex> ent_<hex> [offset].".to_owned());
            return vec![Effect::None];
        };
    let offset = match arguments.next() {
        Some(value) => {
            if let Ok(offset) = value.parse::<usize>() {
                offset
            } else {
                state.conversation_composer.error =
                    Some("Member offset must be a non-negative integer.".to_owned());
                return vec![Effect::None];
            }
        }
        None => 0,
    };
    if arguments.next().is_some() {
        state.conversation_composer.error =
            Some("Usage: /artifact-members prj_<hex> ent_<hex> [offset].".to_owned());
        return vec![Effect::None];
    }
    super::reduce(
        state,
        Action::ArtifactMembersRequested {
            project,
            artifact,
            offset,
        },
    )
}
