//! Executes TUI operations through the shared service contract.

#![allow(clippy::wildcard_imports)]

use super::*;
use crate::action::Action;

impl<S: TuiService> TuiOperationService for S {
    /// Uses the operation method exposed by the TUI service boundary.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `request`: Public operation request.
    ///
    /// # Returns
    /// The operation result.
    ///
    /// # Errors
    /// Returns the service operation error.
    fn execute_operation(
        &self,
        context: &ApplicationContext,
        request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        TuiService::execute_operation(self, context, request)
    }
}

/// Executes one public operation through the TUI operation port.
///
/// # Parameters
/// - `service`: TUI operation service.
/// - `context`: Application capability context.
/// - `request`: Operation request to execute.
///
/// # Returns
/// The application operation result.
///
/// # Errors
/// Returns the application error produced by the operation service.
pub fn execute_operation<S: TuiOperationService>(
    service: &S,
    context: &ApplicationContext,
    request: &ApplicationRequest,
) -> Result<ApplicationResult, ApplicationError> {
    service.execute_operation(context, request)
}

/// Executes a non-inspection effect and converts its public result to an action.
pub fn execute_operation_effect<S: TuiOperationService>(
    service: &S,
    context: &ApplicationContext,
    effect: Effect,
) -> Action {
    let request_id = match &effect {
        Effect::History { request_id, .. }
        | Effect::Diff { request_id, .. }
        | Effect::Render { request_id, .. }
        | Effect::Apply { request_id, .. }
        | Effect::Commit { request_id, .. } => *request_id,
        _ => return Action::RefreshRequested,
    };
    let request = match effect {
        Effect::History { snapshot, .. } => {
            scriberig_app::ApplicationRequest::History(scriberig_app::HistoryRequest {
                snapshot: scriberig_app::SnapshotSelector::Exact(snapshot),
            })
        }
        Effect::Diff { request, .. } => scriberig_app::ApplicationRequest::Diff(request),
        Effect::Render { request, .. } => scriberig_app::ApplicationRequest::Render(request),
        Effect::Apply { request, .. } => scriberig_app::ApplicationRequest::Apply(request),
        Effect::Commit { request, .. } => scriberig_app::ApplicationRequest::Commit(request),
        _ => unreachable!(),
    };
    match service.execute_operation(context, &request) {
        Ok(scriberig_app::ApplicationResult::History(summary)) => {
            let snapshot = summary.snapshot.unwrap_or_else(|| match &request {
                scriberig_app::ApplicationRequest::History(value) => match value.snapshot {
                    scriberig_app::SnapshotSelector::Exact(snapshot) => snapshot,
                    scriberig_app::SnapshotSelector::NamedRoot(_) => {
                        scriberig_core::SnapshotId::new([0; 32])
                    }
                },
                _ => scriberig_core::SnapshotId::new([0; 32]),
            });
            Action::HistorySucceeded {
                request_id,
                model: crate::view::history::HistoryModel::from_summary(snapshot, &summary),
            }
        }
        Ok(scriberig_app::ApplicationResult::Diff(summary)) => summary.diff.as_ref().map_or_else(
            || Action::OperationFailed {
                request_id,
                category: "invalid_result".to_owned(),
                message: "diff result did not contain semantic changes".to_owned(),
            },
            |diff| Action::DiffSucceeded {
                request_id,
                model: crate::view::diff::DiffModel::from_diff(diff),
            },
        ),
        Ok(scriberig_app::ApplicationResult::Render(summary)) => {
            summary.rendered.as_ref().map_or_else(
                || Action::OperationFailed {
                    request_id,
                    category: "invalid_result".to_owned(),
                    message: "render result did not contain projection content".to_owned(),
                },
                |rendered| Action::ProjectionSucceeded {
                    request_id,
                    model: crate::view::projection::ProjectionModel::from_rendered(rendered),
                },
            )
        }
        Ok(scriberig_app::ApplicationResult::Apply(summary)) => {
            let scriberig_app::ApplicationRequest::Apply(request) = request else {
                unreachable!()
            };
            Action::ReviewSucceeded {
                request_id,
                model: crate::view::changes::ChangeReviewModel::from_summary(
                    request.base_snapshot,
                    request.change,
                    request.branch,
                    &summary,
                ),
            }
        }
        Ok(scriberig_app::ApplicationResult::Commit(summary)) => {
            let scriberig_app::ApplicationRequest::Commit(request) = request else {
                unreachable!()
            };
            Action::ReviewSucceeded {
                request_id,
                model: crate::view::changes::ChangeReviewModel::from_summary(
                    request.base_snapshot,
                    request.change,
                    request.branch,
                    &summary,
                ),
            }
        }
        Ok(_) => Action::OperationFailed {
            request_id,
            category: "invalid_result".to_owned(),
            message: "application returned an unexpected operation result".to_owned(),
        },
        Err(error) => Action::OperationFailed {
            request_id,
            category: error.category().to_owned(),
            message: crate::inspector::sanitize(&error.to_string()),
        },
    }
}
