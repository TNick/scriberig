//! Builds the project-source projection request the local TUI view opens.

use scriberig_core::{EntityId, SnapshotId};

/// Project-scope label used when no project workspace has been activated.
///
/// A render selection must always name the scope it resolves against, so an
/// unactivated view names this label instead of a plausible project name. No
/// policy is stored under it, so the resolution stops at the domain default.
const UNSELECTED_PROJECT_SCOPE: &str = "unselected-project";

/// Workspace-scope label used when no project workspace has been activated.
const UNSELECTED_WORKSPACE_SCOPE: &str = "unselected-workspace";

/// Builds effective-style resolution for the exact interactive source view.
///
/// - `entity`: Selected source entity used to identify the personal view.
/// - `project`: Activated project identity, when present.
///
/// # Returns
/// The shared style resolver request matching the rendered source view scopes.
pub(crate) fn style_resolution_request(
    entity: EntityId,
    project: Option<&str>,
) -> scriberig_protocol::ProjectionStyleResolveRequest {
    let (project_id, workspace_id) = match project {
        Some(project) => (project.to_owned(), project.to_owned()),
        None => (
            UNSELECTED_PROJECT_SCOPE.to_owned(),
            UNSELECTED_WORKSPACE_SCOPE.to_owned(),
        ),
    };
    scriberig_protocol::ProjectionStyleResolveRequest {
        projection_kind: "rust".to_owned(),
        project_id,
        workspace_id,
        purpose: scriberig_protocol::ProjectionStylePurpose::InteractiveView,
        view_id: Some(format!("project-source:{entity}")),
        explicit_personal: None,
    }
}

/// Builds one project-source render request for one selected entity.
///
/// The request carries an explicit style selection rather than a style value: the
/// application resolves it through the domain style adapter the kind belongs to,
/// so the surface can never style a projection on its own. The selection is named
/// after the activated project workspace, which is the only scope identity the
/// local TUI tracks: activation records the workspace identity in
/// [`crate::state::TuiState::project`] and there is no separate workspace
/// identity in local state, so both style scopes resolve against that one name.
/// An unactivated view resolves against the unselected-scope labels rather than
/// inventing an identity.
///
/// - `snapshot`: Exact snapshot being projected.
/// - `entity`: Selected entity identity.
/// - `project`: Activated project workspace identity, when one is selected.
///
/// # Returns
/// The render request for that entity's source view.
#[must_use]
pub(crate) fn project_source_request(
    snapshot: SnapshotId,
    entity: EntityId,
    project: Option<&str>,
) -> scriberig_app::RenderRequest {
    let (project_scope, workspace_scope) = match project {
        Some(project) => (project.to_owned(), project.to_owned()),
        None => (
            UNSELECTED_PROJECT_SCOPE.to_owned(),
            UNSELECTED_WORKSPACE_SCOPE.to_owned(),
        ),
    };
    scriberig_app::RenderRequest {
        snapshot: scriberig_app::SnapshotSelector::Exact(snapshot),
        view: format!("project-source:{entity}"),
        style: scriberig_app::RenderStyleSelection::configured(
            // The kind label the Rust domain style adapter owns.
            "rust",
            project_scope,
            workspace_scope,
            scriberig_app::RenderStylePurpose::InteractiveView,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::Action;
    use crate::effect::Effect;
    use crate::state::{TuiState, reduce};

    /// Names one selected entity and snapshot for every case in this module.
    ///
    /// # Returns
    /// The selected entity and exact snapshot used by the projection view.
    fn selected() -> (EntityId, SnapshotId) {
        (EntityId::from_bytes([71; 16]), SnapshotId::new([72; 32]))
    }

    /// Verifies the selection names the activated project workspace for both
    /// style scopes instead of a local placeholder.
    #[test]
    fn selection_names_the_activated_workspace() {
        let (entity, snapshot) = selected();
        let request = project_source_request(snapshot, entity, Some("warehouse-service"));

        let scriberig_app::RenderStyleSelection::Configured {
            projection_kind,
            project_id,
            workspace_id,
            purpose,
            ..
        } = request.style
        else {
            panic!("the local view opens a configured selection");
        };
        assert_eq!(projection_kind, "rust");
        assert_eq!(project_id, "warehouse-service");
        assert_eq!(workspace_id, "warehouse-service");
        assert_eq!(purpose, scriberig_app::RenderStylePurpose::InteractiveView);
    }

    /// Verifies a view opened without an activated workspace names the documented
    /// unselected scope rather than a name that could collide with a real scope.
    #[test]
    fn selection_names_the_unselected_scope_without_a_workspace() {
        let (entity, snapshot) = selected();
        let request = project_source_request(snapshot, entity, None);

        let scriberig_app::RenderStyleSelection::Configured {
            project_id,
            workspace_id,
            ..
        } = request.style
        else {
            panic!("the local view opens a configured selection");
        };
        assert_eq!(project_id, UNSELECTED_PROJECT_SCOPE);
        assert_eq!(workspace_id, UNSELECTED_WORKSPACE_SCOPE);
    }

    /// Verifies opening a projection through the reducer carries the active
    /// project identity, so the resolved style belongs to the project on screen.
    #[test]
    fn projection_opened_carries_the_active_project_identity() {
        let (entity, snapshot) = selected();
        let mut state = TuiState {
            project: Some("warehouse-service".to_owned()),
            selection: Some(crate::selection::EntitySelection {
                entity,
                revision: None,
                snapshot: Some(snapshot),
            }),
            ..TuiState::default()
        };

        let effects = reduce(&mut state, Action::ProjectionOpened);

        let [Effect::Render { request, .. }] = effects.as_slice() else {
            panic!("opening a projection requests one render: {effects:?}");
        };
        let scriberig_app::RenderStyleSelection::Configured { project_id, .. } = &request.style
        else {
            panic!("the local view opens a configured selection");
        };
        assert_eq!(project_id, "warehouse-service");
    }
}
