//! Maps remote operation records, including structured agent results.

use std::str::FromStr;

use super::parse_entity;
use super::parse_snapshot;
use scriberig_app::ApplicationError;

/// Converts one protocol operation into the shared application lifecycle model.
///
/// # Parameters
/// - `operation`: Remote operation record.
///
/// # Returns
/// The shared lifecycle record, including any structured agent result.
///
/// # Errors
/// Returns an error when a remote identity is malformed.
/// - `crate`: Internal visibility marker.
pub(crate) fn map_remote_operation(
    operation: scriberig_protocol::Operation,
) -> Result<scriberig_app::OperationRecord, ApplicationError> {
    let reference = scriberig_core::OperationId::from_str(&operation.reference.id)
        .or_else(|_| scriberig_core::OperationId::new(operation.reference.id.clone()))
        .map_err(|error| ApplicationError::Internal {
            message: format!("remote response contained invalid operation identity: {error}"),
        })?;
    let kind = match operation.kind {
        scriberig_protocol::operation::OperationKind::Inspect => {
            scriberig_app::OperationKind::Inspect
        }
        scriberig_protocol::operation::OperationKind::Query => scriberig_app::OperationKind::Query,
        scriberig_protocol::operation::OperationKind::List => scriberig_app::OperationKind::List,
        scriberig_protocol::operation::OperationKind::Follow => {
            scriberig_app::OperationKind::Follow
        }
        scriberig_protocol::operation::OperationKind::Find => scriberig_app::OperationKind::Find,
        scriberig_protocol::operation::OperationKind::Diff => scriberig_app::OperationKind::Diff,
        scriberig_protocol::operation::OperationKind::History => {
            scriberig_app::OperationKind::History
        }
        scriberig_protocol::operation::OperationKind::Render => {
            scriberig_app::OperationKind::Render
        }
        scriberig_protocol::operation::OperationKind::Branch => {
            scriberig_app::OperationKind::Branch
        }
        scriberig_protocol::operation::OperationKind::Agent => scriberig_app::OperationKind::Agent,
        scriberig_protocol::operation::OperationKind::Apply => scriberig_app::OperationKind::Apply,
        scriberig_protocol::operation::OperationKind::Commit => {
            scriberig_app::OperationKind::Commit
        }
        scriberig_protocol::operation::OperationKind::Validate
        | scriberig_protocol::operation::OperationKind::Promote
        | scriberig_protocol::operation::OperationKind::Custom(_) => {
            scriberig_app::OperationKind::Query
        }
    };
    let status = match operation.status {
        scriberig_protocol::OperationStatus::Queued => scriberig_app::OperationStatus::Queued,
        scriberig_protocol::OperationStatus::Running => scriberig_app::OperationStatus::Running,
        scriberig_protocol::OperationStatus::Completed => scriberig_app::OperationStatus::Completed,
        scriberig_protocol::OperationStatus::Failed => scriberig_app::OperationStatus::Failed,
        scriberig_protocol::OperationStatus::Cancelled => scriberig_app::OperationStatus::Cancelled,
    };
    let mut record = scriberig_app::OperationRecord::queued(reference, kind);
    record.status = status;
    record.result_handles = operation.result_handles;
    record.progress = operation
        .progress
        .map(|progress| scriberig_app::OperationProgress {
            status,
            completed: Some(u64::from(progress.percent)),
            total: Some(100),
            phase: progress.message,
        });
    if let Some(error) = operation.error {
        record.error = Some(ApplicationError::OperationFailed {
            operation: record.reference.to_string(),
            message: crate::inspector::sanitize(&error),
        });
    }
    record.agent_result = operation.agent_result.map(map_agent_result).transpose()?;
    Ok(record)
}

/// Converts one protocol agent result into the application result model.
///
/// # Parameters
/// - `result`: Remote terminal agent result.
///
/// # Returns
/// Parsed provider-neutral agent result.
///
/// # Errors
/// Returns an error when any semantic handle is malformed.
fn map_agent_result(
    result: scriberig_protocol::AgentResult,
) -> Result<scriberig_app::AgentResult, ApplicationError> {
    Ok(scriberig_app::AgentResult {
        session: parse_entity(&result.session.id)?,
        root_branch: parse_entity(&result.root_branch.id)?,
        final_output: result.final_output,
        branch_result: parse_entity(&result.branch_result)?,
        produced_changes: result
            .produced_changes
            .iter()
            .map(|change| parse_entity(&change.id))
            .collect::<Result<Vec<_>, _>>()?,
        validation_results: result.validation_results,
        unresolved_questions: result.unresolved_questions,
        candidate_changes: result
            .candidate_changes
            .into_iter()
            .map(|change| {
                Ok(scriberig_app::CandidateChange {
                    migration: change.migration,
                    entity: change
                        .entity
                        .map(|entity| parse_entity(&entity.id))
                        .transpose()?,
                    candidate_snapshot: change
                        .candidate_snapshot
                        .map(|snapshot| parse_snapshot(&snapshot.id))
                        .transpose()?,
                    validation: change.validation,
                })
            })
            .collect::<Result<Vec<_>, ApplicationError>>()?,
    })
}
