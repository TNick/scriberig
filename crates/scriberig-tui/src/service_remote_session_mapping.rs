//! Restores shared session state and validates remote semantic identities.

use std::str::FromStr;

use super::{ApplicationError, EntityId, plan_mapping, remote_activity};

/// Converts a compact protocol session response to the shared session model.
///
/// - `response`: Protocol session tree response.
///
/// Session value accepted by the existing TUI projection.
///
/// Returns an internal error when a remote reference is malformed.
/// - `crate`: Internal visibility marker.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(crate) fn session_from_response(
    response: scriberig_protocol::SessionReadResponse,
) -> Result<scriberig_session::Session, ApplicationError> {
    let id = parse_entity(&response.session.id)?;
    let branches = response
        .branches
        .into_iter()
        .map(|value| {
            let branch_id = parse_entity(&value.id)?;
            let mut branch =
                scriberig_session::Branch::new(branch_id, value.title).map_err(|error| {
                    ApplicationError::Internal {
                        message: format!("remote branch was invalid: {error}"),
                    }
                })?;
            branch.parent = value.parent.as_deref().map(parse_entity).transpose()?;
            branch.children = value
                .children
                .iter()
                .map(|child| parse_entity(child))
                .collect::<Result<Vec<_>, _>>()?;
            let status = match value.status {
                scriberig_protocol::SessionBranchStatus::Active => {
                    scriberig_session::BranchStatus::Active
                }
                scriberig_protocol::SessionBranchStatus::Completed => {
                    scriberig_session::BranchStatus::Completed
                }
                scriberig_protocol::SessionBranchStatus::Abandoned => {
                    scriberig_session::BranchStatus::Abandoned
                }
            };
            branch
                .transition(status)
                .map_err(|error| ApplicationError::Internal {
                    message: format!("remote branch status was invalid: {error}"),
                })?;
            branch.turns = value
                .turns
                .into_iter()
                .map(|turn| {
                    Ok(scriberig_session::Turn {
                        id: parse_entity(&turn.id)?,
                        ordinal: turn.ordinal,
                        human: turn.has_human_input.then(|| scriberig_session::HumanInput {
                            text: String::new(),
                        }),
                        agent: turn
                            .has_agent_output
                            .then(|| scriberig_session::AgentOutput {
                                text: turn.agent_output.unwrap_or_default(),
                            }),
                        tool_invocation: turn
                            .tool_invocation
                            .as_deref()
                            .map(parse_entity)
                            .transpose()?,
                        tool_result: turn.tool_result.as_deref().map(parse_entity).transpose()?,
                    })
                })
                .collect::<Result<Vec<_>, ApplicationError>>()?;
            branch.result = value
                .result
                .map(|result| {
                    Ok(scriberig_session::BranchResult {
                        id: parse_entity(&result.id)?,
                        branch: branch_id,
                        status: scriberig_session::BranchStatus::Completed,
                        summary: None,
                        result_kind: None,
                        candidate_context: None,
                        references: result
                            .references
                            .iter()
                            .map(|reference| parse_entity(reference))
                            .collect::<Result<Vec<_>, _>>()?,
                        result_handles: result.result_handles,
                        touched: Vec::new(),
                        produced_changes: Vec::new(),
                        validations: Vec::new(),
                        unresolved_questions: Vec::new(),
                    })
                })
                .transpose()?;
            remote_activity::restore_activity(&mut branch, value.activity)?;
            Ok(branch)
        })
        .collect::<Result<Vec<_>, ApplicationError>>()?;
    let plans = response
        .plans
        .into_iter()
        .map(plan_mapping::from_response)
        .collect::<Result<Vec<_>, ApplicationError>>()?;
    Ok(scriberig_session::Session {
        id,
        branches,
        goals: Vec::new(),
        tasks: Vec::new(),
        plans,
        execution: execution_from_response(response.execution),
    })
}

/// Parses the structural parent identity carried by the current remote DTO.
///
/// - `value`: Tagged `branch:`/`node:` parent or legacy branch identity.
///
/// The typed structural parent, or `None` when the field is empty.
///
/// Returns an internal error when the tagged identity is malformed.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn parse_plan_parent(
    value: &str,
) -> Result<Option<scriberig_session::PlanNodeParent>, ApplicationError> {
    if let Some(branch) = value.strip_prefix("branch:") {
        return parse_entity(branch)
            .map(scriberig_session::PlanNodeParent::Branch)
            .map(Some);
    }
    if let Some(node) = value.strip_prefix("node:") {
        return parse_entity(node)
            .map(scriberig_session::PlanNodeParent::Node)
            .map(Some);
    }
    if value.is_empty() {
        return Ok(None);
    }
    parse_entity(value)
        .map(scriberig_session::PlanNodeParent::Branch)
        .map(Some)
}

/// Converts the public execution selection and history into the session owner.
///
/// - `selection`: Protocol selection carried by the session response.
///
/// Equivalent session-domain execution selection.
/// # Returns
/// The operation result.
fn execution_from_response(
    selection: scriberig_protocol::SessionExecutionSelection,
) -> scriberig_session::SessionExecutionSelection {
    scriberig_session::SessionExecutionSelection {
        revision: selection.revision,
        provider_id: selection.provider_id,
        model_id: selection.model_id,
        thinking_level: selection.thinking_level,
        history: selection
            .history
            .into_iter()
            .map(|change| scriberig_session::SessionExecutionChange {
                revision: change.revision,
                kind: change.kind,
                summary: change.summary,
                provider_id: change.provider_id,
                model_id: change.model_id,
                thinking_level: change.thinking_level,
            })
            .collect(),
    }
}

/// Parses one remote entity identity and sanitizes malformed responses.
///
/// - `value`: Remote entity identity text.
///
/// Parsed entity identity.
///
/// Returns an internal error for malformed remote identity text.
/// - `crate`: Internal visibility marker.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(crate) fn parse_entity(value: &str) -> Result<EntityId, ApplicationError> {
    EntityId::from_str(value).map_err(|error| ApplicationError::Internal {
        message: format!("remote response contained invalid entity identity: {error}"),
    })
}

/// Parses one remote snapshot identity and sanitizes malformed responses.
///
/// - `value`: Remote snapshot identity text.
///
/// Parsed snapshot identity.
///
/// Returns an internal error for malformed remote identity text.
/// - `super`: Internal visibility marker.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn parse_snapshot(value: &str) -> Result<scriberig_core::SnapshotId, ApplicationError> {
    scriberig_core::SnapshotId::from_str(value).map_err(|error| ApplicationError::Internal {
        message: format!("remote response contained invalid snapshot identity: {error}"),
    })
}
