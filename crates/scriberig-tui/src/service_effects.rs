//! Maps TUI effects to shared application operations and actions.

use std::collections::BTreeSet;

use super::{
    ApplicationContext, ApplicationError, ApplicationRequest, ApplicationResult,
    execute_operation_effect,
};
use crate::{action::Action, effect::Effect, service::TuiService};
use scriberig_core::EntityId;
use scriberig_session::Session;

#[path = "service_effects_recovery.rs"]
mod recovery;
#[path = "service_effects_rich.rs"]
mod rich;

/// Supplies shared application operations to semantic TUI views.
pub trait TuiOperationService {
    /// Executes one public application operation without exposing storage.
    ///
    /// - `context`: Authorized application context.
    /// - `request`: Public universal application request.
    ///
    /// Structured application result owned by the application layer.
    ///
    /// Returns the structured application error from the owning subsystem.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
    let effect = match rich::execute(service, context, effect) {
        Ok(action) => return action,
        Err(effect) => *effect,
    };
    match effect {
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
                conversation: crate::view::conversation::ConversationModel::from_session(&value),
                session_snapshot: Some(value),
            })
            .unwrap_or_else(|error| Action::OperationFailed {
                request_id,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        effect @ (Effect::LegacyRecoveryList { .. } | Effect::LegacyRecoveryMap { .. }) => {
            recovery::execute(service, context, effect)
        }
        Effect::ListSessions => service
            .list_sessions(context)
            .map(Action::SessionsListed)
            .unwrap_or_else(|error| Action::OperationFailed {
                request_id: 0,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::SwitchSession { session } => service
            .read_session(context, session)
            .map(Action::SessionSwitched)
            .unwrap_or_else(|error| Action::OperationFailed {
                request_id: 0,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::RemoveSession { session } => service
            .remove_session(context, session)
            .map(|()| Action::SessionRemoved(session))
            .unwrap_or_else(|error| Action::OperationFailed {
                request_id: 0,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::ReadModelLibrary { request_id } => service
            .model_library(context)
            .map(|library| Action::ModelLibraryLoaded {
                request_id,
                library,
            })
            .unwrap_or_else(|error| Action::OperationFailed {
                request_id,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::ReadPresentationPreferences => service
            .agent_presentation_preferences(context)
            .map(Action::PresentationPreferencesLoaded)
            .unwrap_or_else(|error| Action::OperationFailed {
                request_id: 0,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::ReadToolResultDetail {
            session,
            branch,
            result,
        } => service
            .tool_result_detail(context, session, branch, result)
            .map(Action::ToolResultDetailLoaded)
            .unwrap_or_else(|error| Action::OperationFailed {
                request_id: 0,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::ReadModelRequestDetail {
            session,
            branch,
            invocation,
        } => service
            .model_request_detail(context, session, branch, invocation)
            .map(Action::ModelRequestDetailLoaded)
            .unwrap_or_else(|error| Action::OperationFailed {
                request_id: 0,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::SetSessionModel(request) => service
            .set_session_model(context, &request)
            .map(Action::SessionExecutionSelectionUpdated)
            .unwrap_or_else(|error| Action::OperationFailed {
                request_id: 0,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::SetSessionThinkingLevel(request) => service
            .set_session_thinking_level(context, &request)
            .map(Action::SessionExecutionSelectionUpdated)
            .unwrap_or_else(|error| Action::OperationFailed {
                request_id: 0,
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
        Effect::StartNewConversation { prompt, selection } => service
            .start_new_session_handle(context, &prompt, selection)
            .map(Action::ConversationAgentAccepted)
            .unwrap_or_else(|error| Action::OperationFailed {
                request_id: 0,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::ContinueConversation {
            session,
            anchor,
            prompt,
        } => service
            .continue_agent_at_handle(context, session, anchor, &prompt)
            .map(Action::ConversationAgentAccepted)
            .unwrap_or_else(|error| Action::OperationFailed {
                request_id: 0,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::CreatePlan(request) => service
            .create_plan(context, &request)
            .map(Action::PlanUpdated)
            .unwrap_or_else(|error| Action::OperationFailed {
                request_id: 0,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::Replan(request) => service
            .replan(context, &request)
            .map(Action::PlanUpdated)
            .unwrap_or_else(|error| Action::OperationFailed {
                request_id: 0,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::PlanLifecycle { request, action } => service
            .plan_lifecycle(context, &request, action)
            .map(Action::PlanUpdated)
            .unwrap_or_else(|error| Action::OperationFailed {
                request_id: 0,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::PlanMutation(request) => service
            .mutate_plan(context, &request)
            .map(Action::PlanUpdated)
            .unwrap_or_else(|error| Action::OperationFailed {
                request_id: 0,
                category: error.category().to_owned(),
                message: crate::inspector::sanitize(&error.to_string()),
            }),
        Effect::SetPlanNodeAssignment(request) => service
            .set_plan_node_assignment(context, &request)
            .map(Action::PlanUpdated)
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
        _ => unreachable!("rich-domain effects are dispatched before this match"),
    }
}

/// Selects one branch and all descendants from an authoritative session.
///
/// - `session`: Session graph to filter.
/// - `branch`: Selected branch identity.
/// - `crate`: Internal visibility marker.
///
/// A session containing the selected branch and descendants.
/// # Returns
/// The operation result.
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
