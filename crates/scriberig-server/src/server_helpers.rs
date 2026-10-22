//! Converts application results and events into protocol values.

use crate::server::{Event, Operation};
use scriberig_app::ApplicationError;

#[path = "server_helpers_activity.rs"]
mod activity;
#[path = "server_helpers_event.rs"]
mod event;
#[path = "server_helpers_session_plan.rs"]
pub(super) mod session_plan;
pub(crate) use session_plan::project as project_session_plan;

pub(super) use event::event_from_application;

#[cfg(test)]
#[path = "server_helpers_session_tests.rs"]
mod session_plan_tests;

/// Converts a durable session into a compact provider-neutral response.
///
/// - `super`: Server protocol module namespace used by the response type.
/// - `session`: Durable session value returned by the application boundary.
///
/// Session tree containing ordered activity and compact result handles.
/// # Returns
/// The operation result.
///
/// # Errors
/// Returns an error when the requested operation cannot complete.
pub(super) fn session_response(
    session: scriberig_session::Session,
) -> Result<scriberig_protocol::SessionReadResponse, scriberig_app::ApplicationError> {
    let session_id = session.id;
    let execution = session.execution;
    let results = session
        .branches
        .iter()
        .filter_map(|branch| branch.result.clone())
        .collect::<Vec<_>>();
    let plans = session
        .plans
        .into_iter()
        .map(|plan| session_plan::project(plan, Some(&results)))
        .collect::<Result<Vec<_>, _>>()?;
    let mut branches = session
        .branches
        .into_iter()
        .map(|branch| {
            let activity = activity::branch_execution_activity(&session_id.to_string(), &branch);
            scriberig_protocol::SessionBranch {
                id: branch.id.to_string(),
                parent: branch.parent.map(|value| value.to_string()),
                children: branch.children.iter().map(ToString::to_string).collect(),
                title: branch.title,
                status: match branch.status {
                    scriberig_session::BranchStatus::Active => {
                        scriberig_protocol::SessionBranchStatus::Active
                    }
                    scriberig_session::BranchStatus::Completed => {
                        scriberig_protocol::SessionBranchStatus::Completed
                    }
                    scriberig_session::BranchStatus::Abandoned => {
                        scriberig_protocol::SessionBranchStatus::Abandoned
                    }
                },
                turns: branch
                    .turns
                    .into_iter()
                    .map(|turn| scriberig_protocol::SessionTurn {
                        id: turn.id.to_string(),
                        ordinal: turn.ordinal,
                        has_human_input: turn.human.is_some(),
                        has_agent_output: turn.agent.is_some(),
                        agent_output: turn.agent.map(|value| value.text),
                        tool_invocation: turn.tool_invocation.map(|value| value.to_string()),
                        tool_result: turn.tool_result.map(|value| value.to_string()),
                    })
                    .collect(),
                activity,
                result: branch
                    .result
                    .map(|result| scriberig_protocol::SessionBranchResult {
                        id: result.id.to_string(),
                        references: result
                            .references
                            .into_iter()
                            .map(|value| value.to_string())
                            .collect(),
                        result_handles: result.result_handles,
                    }),
            }
        })
        .collect::<Vec<_>>();
    branches.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(scriberig_protocol::SessionReadResponse {
        session: scriberig_protocol::SessionReference {
            id: session_id.to_string(),
        },
        branches,
        plans,
        execution: scriberig_protocol::SessionExecutionSelection {
            revision: execution.revision,
            provider_id: execution.provider_id,
            model_id: execution.model_id,
            thinking_level: execution.thinking_level,
            history: execution
                .history
                .into_iter()
                .map(|change| scriberig_protocol::SessionExecutionChange {
                    revision: change.revision,
                    kind: change.kind,
                    summary: change.summary,
                    provider_id: change.provider_id,
                    model_id: change.model_id,
                    thinking_level: change.thinking_level,
                })
                .collect(),
        },
    })
}

/// Extracts stable result handles without exposing canonical result payloads.
/// - `super`: Parent-module imports used by this helper.
/// - `result`: Argument named `result`.
///
/// # Returns
/// The operation result.
pub(super) fn result_handles(result: &scriberig_app::ApplicationResult) -> Vec<String> {
    if let scriberig_app::ApplicationResult::DomainOperation(value) = result {
        return vec![value.change.clone()];
    }
    let summary = match result {
        scriberig_app::ApplicationResult::Diff(value)
        | scriberig_app::ApplicationResult::History(value)
        | scriberig_app::ApplicationResult::Render(value)
        | scriberig_app::ApplicationResult::Branch(value)
        | scriberig_app::ApplicationResult::Apply(value)
        | scriberig_app::ApplicationResult::Commit(value) => Some(value),
        _ => None,
    };
    summary.map_or_else(Vec::new, |value| vec![value.handle.clone()])
}

/// Tests whether a protocol event matches a case-insensitive kind filter.
/// - `super`: Parent-module imports used by this helper.
///
/// - `event`: Event whose kind is being tested.
/// - `event_kinds`: Requested event kind names; empty selects every event.
///
/// `true` when the event is selected by the subscription.
/// # Returns
/// The operation result.
pub(super) fn event_matches(event: &Event, event_kinds: &[String]) -> bool {
    event_kinds.is_empty()
        || event_kinds.iter().any(|kind| {
            kind.eq_ignore_ascii_case(&format!("{:?}", event.kind))
                || kind.eq_ignore_ascii_case(&format!("{:?}", event.kind).replace('_', ""))
        })
}

/// Preserves safe semantic handles in protocol error details.
///
/// - `error`: Application error being serialized.
///
/// JSON-safe structured details without backend internals.
/// # Returns
/// The operation result.
pub fn error_details(error: &ApplicationError) -> serde_json::Value {
    match error {
        ApplicationError::NotFound { subject } => serde_json::json!({"subject": subject}),
        ApplicationError::InvalidRequest { operation, message }
        | ApplicationError::OperationFailed { operation, message } => {
            serde_json::json!({"operation": operation, "message": message})
        }
        ApplicationError::UnsupportedCapability {
            operation,
            capability,
        } => serde_json::json!({"operation": operation, "capability": capability}),
        ApplicationError::Conflict { subject, details } => serde_json::json!({
            "subject": subject,
            "base_revision": details.base_revision.map(|value| value.to_string()),
            "expected_revision": details.expected_revision.map(|value| value.to_string()),
            "current_revision": details.current_revision.map(|value| value.to_string()),
            "left": details.left.as_ref(),
            "right": details.right.as_ref(),
            "operation": details.operation,
        }),
        ApplicationError::Ambiguous {
            scope,
            candidates,
            reason,
        } => serde_json::json!({"scope": scope, "candidates": candidates, "reason": reason}),
        ApplicationError::SnapshotUnavailable { snapshot } => {
            serde_json::json!({"snapshot": snapshot.to_string()})
        }
        _ => serde_json::json!({}),
    }
}

/// Converts an application operation record to its public lifecycle form.
/// - `super`: Parent-module imports used by this helper.
/// - `record`: Argument named `record`.
///
/// # Returns
/// The operation result.
pub(crate) fn operation_from_record(record: scriberig_app::OperationRecord) -> Operation {
    Operation {
        reference: scriberig_protocol::OperationReference {
            id: record.reference.to_string(),
        },
        kind: match record.kind {
            scriberig_app::OperationKind::Inspect => {
                scriberig_protocol::operation::OperationKind::Inspect
            }
            scriberig_app::OperationKind::Query => {
                scriberig_protocol::operation::OperationKind::Query
            }
            scriberig_app::OperationKind::Apply => {
                scriberig_protocol::operation::OperationKind::Apply
            }
            scriberig_app::OperationKind::List => {
                scriberig_protocol::operation::OperationKind::List
            }
            scriberig_app::OperationKind::Follow => {
                scriberig_protocol::operation::OperationKind::Follow
            }
            scriberig_app::OperationKind::Find => {
                scriberig_protocol::operation::OperationKind::Find
            }
            scriberig_app::OperationKind::Diff => {
                scriberig_protocol::operation::OperationKind::Diff
            }
            scriberig_app::OperationKind::History => {
                scriberig_protocol::operation::OperationKind::History
            }
            scriberig_app::OperationKind::Render => {
                scriberig_protocol::operation::OperationKind::Render
            }
            scriberig_app::OperationKind::Branch => {
                scriberig_protocol::operation::OperationKind::Branch
            }
            scriberig_app::OperationKind::Agent => {
                scriberig_protocol::operation::OperationKind::Agent
            }
            scriberig_app::OperationKind::Plan => {
                scriberig_protocol::operation::OperationKind::Plan
            }
            scriberig_app::OperationKind::DomainOperation => {
                scriberig_protocol::operation::OperationKind::DomainOperation
            }
            scriberig_app::OperationKind::Commit => {
                scriberig_protocol::operation::OperationKind::Commit
            }
        },
        status: match record.status {
            scriberig_app::OperationStatus::Queued => {
                scriberig_protocol::operation::OperationStatus::Queued
            }
            scriberig_app::OperationStatus::Running => {
                scriberig_protocol::operation::OperationStatus::Running
            }
            scriberig_app::OperationStatus::Completed => {
                scriberig_protocol::operation::OperationStatus::Completed
            }
            scriberig_app::OperationStatus::Failed => {
                scriberig_protocol::operation::OperationStatus::Failed
            }
            scriberig_app::OperationStatus::Cancelled => {
                scriberig_protocol::operation::OperationStatus::Cancelled
            }
        },
        progress: record.progress.map(|progress| {
            let percent = progress
                .total
                .filter(|total| *total > 0)
                .map_or(0, |total| {
                    progress
                        .completed
                        .unwrap_or_default()
                        .saturating_mul(100)
                        .checked_div(total)
                        .unwrap_or(0)
                        .min(100) as u8
                });
            scriberig_protocol::OperationProgress {
                percent,
                message: progress.phase,
            }
        }),
        result_snapshot: None,
        result_handles: record.result_handles,
        agent_result: record.agent_result.map(agent_result),
        error: record.error.map(|error| error.to_string()),
    }
}

/// Converts an application agent result into a provider-neutral protocol value.
///
/// - `result`: Durable application result.
///
/// Safe protocol result without provider credentials or DTOs.
///
/// This conversion is infallible because all identifiers have already been
/// validated by the application layer.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn agent_result(result: scriberig_app::AgentResult) -> scriberig_protocol::AgentResult {
    scriberig_protocol::AgentResult {
        session: scriberig_protocol::SessionReference {
            id: result.session.to_string(),
        },
        root_branch: scriberig_protocol::BranchReference {
            id: result.root_branch.to_string(),
        },
        final_output: result.final_output,
        branch_result: result.branch_result.to_string(),
        produced_changes: result
            .produced_changes
            .into_iter()
            .map(|value| scriberig_protocol::EntityReference {
                id: value.to_string(),
            })
            .collect(),
        validation_results: result.validation_results,
        unresolved_questions: result.unresolved_questions,
        candidate_changes: result
            .candidate_changes
            .into_iter()
            .map(|value| scriberig_protocol::CandidateChangeReference {
                migration: value.migration,
                entity: value
                    .entity
                    .map(|id| scriberig_protocol::EntityReference { id: id.to_string() }),
                candidate_snapshot: value
                    .candidate_snapshot
                    .map(|id| scriberig_protocol::SnapshotReference { id: id.to_string() }),
                validation: value.validation,
            })
            .collect(),
    }
}
