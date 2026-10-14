//! Maps TUI effects to shared application operations and actions.

use std::collections::BTreeSet;

use super::{
    ApplicationContext, ApplicationError, ApplicationRequest, ApplicationResult,
    execute_operation_effect,
};
use crate::{action::Action, effect::Effect, inspector::InspectorModel, service::TuiService};
use scriberig_core::EntityId;
use scriberig_session::Session;

/// Supplies shared application operations to semantic TUI views.
pub trait TuiOperationService {
    /// Executes one public application operation without exposing storage.
    ///
    /// # Parameters
    /// - `context`: Authorized application context.
    /// - `request`: Public universal application request.
    ///
    /// # Returns
    /// Structured application result owned by the application layer.
    ///
    /// # Errors
    /// Returns the structured application error from the owning subsystem.
    fn execute_operation(
        &self,
        context: &ApplicationContext,
        request: &ApplicationRequest,
    ) -> Result<scriberig_app::ApplicationResult, ApplicationError>;
}

/// Adapts the complete public application facade for operation-backed views.
#[allow(clippy::too_many_lines)]
pub fn execute_effect<S: TuiService>(
    service: &S,
    context: &ApplicationContext,
    effect: Effect,
) -> Action {
    match effect {
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
        Effect::RequestCapabilities => match service.runtime_capabilities(context) {
            Ok(capabilities) => Action::CapabilitiesLoaded(capabilities),
            Err(error) => Action::InspectFailed {
                request_id: 0,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            },
        },
        Effect::History { .. }
        | Effect::Diff { .. }
        | Effect::Render { .. }
        | Effect::Apply { .. }
        | Effect::Commit { .. } => execute_operation_effect(service, context, effect),
        Effect::OperationStatus { operation } => service
            .operation_status(context, &operation)
            .map(|record| {
                Action::OperationObserved(crate::view::operations::OperationModel::from_record(
                    &record,
                ))
            })
            .unwrap_or_else(|error| Action::OperationFailed {
                request_id: 0,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::CancelOperation { operation } => service
            .cancel_operation(context, &operation)
            .map(|record| {
                Action::OperationObserved(crate::view::operations::OperationModel::from_record(
                    &record,
                ))
            })
            .unwrap_or_else(|error| Action::OperationFailed {
                request_id: 0,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::ListOperations => service
            .operation_records(context)
            .map(Action::OperationsLoaded)
            .unwrap_or_else(|error| Action::OperationFailed {
                request_id: 0,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::ReadSession {
            request_id,
            session,
            branch,
        } => branch
            .map_or_else(
                || service.read_session(context, session),
                |branch| service.read_session_branch(context, session, branch),
            )
            .map(|value| Action::SessionSucceeded {
                request_id,
                model: crate::view::session::SessionModel::from_session(&value),
            })
            .unwrap_or_else(|error| Action::OperationFailed {
                request_id,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::SubmitAgent {
            session,
            branch,
            prompt,
        } => service
            .submit_agent_handle(context, session, branch, &prompt)
            .map(Action::AgentSubmitted)
            .unwrap_or_else(|error| Action::OperationFailed {
                request_id: 0,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::ProjectAgent {
            request_id,
            entity,
            snapshot,
            prompt,
        } => match service.project_agent(context, entity, snapshot, &prompt) {
            Ok(ApplicationResult::Apply(summary)) => Action::ReviewSucceeded {
                request_id,
                model: crate::view::changes::ChangeReviewModel::from_summary(
                    snapshot,
                    summary.handle.clone(),
                    None,
                    &summary,
                ),
            },
            Ok(_) => Action::OperationFailed {
                request_id,
                category: "invalid_result".to_owned(),
                message: "project agent did not produce a candidate".to_owned(),
            },
            Err(error) => Action::OperationFailed {
                request_id,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            },
        },
        Effect::RequestRefresh => Action::RefreshRequested,
        Effect::RequestQuit => Action::QuitRequested,
        Effect::RequestFind {
            request_id,
            text,
            snapshot,
        } => service
            .find_entities(context, snapshot, &text)
            .map(|model| Action::FindSucceeded { request_id, model })
            .unwrap_or_else(|error| Action::OperationFailed {
                request_id: 0,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::RequestSemanticAction { id, .. } => Action::OperationFailed {
            request_id: 0,
            category: "unsupported".to_owned(),
            message: format!("semantic action is not available through this TUI service: {id}"),
        },
        Effect::None => Action::Noop,
    }
}

/// Selects one branch and all descendants from an authoritative session.
///
/// # Parameters
/// - `session`: Session graph to filter.
/// - `branch`: Selected branch identity.
/// - `crate`: Internal visibility marker.
///
/// # Returns
/// A session containing the selected branch and descendants.
pub(crate) fn select_branch_subtree(mut session: Session, branch: EntityId) -> Session {
    let mut selected = BTreeSet::from([branch]);
    let mut changed = true;
    while changed {
        changed = false;
        for candidate in &session.branches {
            if candidate
                .parent
                .is_some_and(|parent| selected.contains(&parent))
                && selected.insert(candidate.id)
            {
                changed = true;
            }
        }
    }
    session
        .branches
        .retain(|candidate| selected.contains(&candidate.id));
    session
}
