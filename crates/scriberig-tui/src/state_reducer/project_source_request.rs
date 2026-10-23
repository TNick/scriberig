//! Builds the project-source projection request the local TUI view opens.

use scriberig_core::{EntityId, SnapshotId};

/// Builds one project-source render request for one selected entity.
///
/// The request carries an explicit style selection rather than a style value: the
/// application resolves it through the domain adapter the kind belongs to, so the
/// surface can never style a projection on its own. Scope identities are the
/// local composition's until task 153 exposes shared style-selection controls.
///
/// - `snapshot`: Exact snapshot being projected.
/// - `entity`: Selected entity identity.
///
/// # Returns
/// The render request for that entity's source view.
#[must_use]
pub(crate) fn project_source_request(
    snapshot: SnapshotId,
    entity: EntityId,
) -> scriberig_app::RenderRequest {
    scriberig_app::RenderRequest {
        snapshot: scriberig_app::SnapshotSelector::Exact(snapshot),
        view: format!("project-source:{entity}"),
        style: scriberig_app::RenderStyleSelection::configured(
            // The kind label the Rust domain style adapter owns.
            "rust",
            "local-project",
            "local-workspace",
            scriberig_app::RenderStylePurpose::InteractiveView,
        ),
    }
}
