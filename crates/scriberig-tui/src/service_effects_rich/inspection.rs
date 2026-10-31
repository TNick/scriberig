//! Executes bounded rich TUI operations for one semantic view family.

use super::super::ApplicationContext;
use crate::{action::Action, effect::Effect, inspector::InspectorModel, service::TuiService};

/// Executes one bounded group of related semantic view operations.
///
/// # Parameters
/// - `service`: Shared TUI service that owns the operation.
/// - `context`: Authorized application context.
/// - `effect`: Candidate effect to dispatch.
///
/// # Returns
/// The resulting action when this group owns the effect.
///
/// # Errors
/// Returns the effect boxed when another handler owns it.
pub(super) fn execute_inspection<S: TuiService>(
    service: &S,
    context: &ApplicationContext,
    effect: Effect,
) -> Result<Action, Box<Effect>> {
    Ok(match effect {
        Effect::InspectEntity {
            request_id,
            entity,
            snapshot,
        } => match service.inspect_entity(context, entity, snapshot) {
            Ok(result) => Action::InspectSucceeded {
                request_id,
                model: InspectorModel::from_result(result),
            },
            Err(error) => Action::InspectFailed {
                request_id,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            },
        },
        Effect::ListExplorer {
            request_id,
            entity,
            snapshot,
            continuation,
        } => service
            .list_explorer(context, entity, snapshot, continuation)
            .map(|page| Action::ExplorerSucceeded {
                request_id,
                rows: page.rows,
                next: page.next,
            })
            .unwrap_or_else(|error| Action::OperationFailed {
                request_id,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::InspectArtifactPage {
            request_id,
            project,
            offset,
            limit,
        } => service
            .inspect_artifact_page(context, project, offset, limit)
            .map(|model| Action::ArtifactsLoaded { request_id, model })
            .unwrap_or_else(|error| Action::ArtifactsFailed {
                request_id,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::InspectArtifactMembers {
            request_id,
            project,
            artifact,
            offset,
            limit,
        } => service
            .inspect_artifact_members(context, project, artifact, offset, limit)
            .map(|model| Action::ArtifactMembersLoaded { request_id, model })
            .unwrap_or_else(|error| Action::ArtifactMembersFailed {
                request_id,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        other => return Err(Box::new(other)),
    })
}
