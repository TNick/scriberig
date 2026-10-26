//! Adapts explicit legacy execution recovery to protocol-safe values.

use super::{parse_session, selection_response};
use scriberig_app::{AgentConsoleBackend, ApplicationBackend, ApplicationContext};

/// Reads preserved, sanitized legacy execution records for recovery.
///
/// - `service`: Shared application service.
/// - `context`: Trusted capability context.
/// - `session`: Serialized stable session identity.
///
/// Safe legacy record views for operator review.
/// # Returns
/// The operation result.
/// # Errors
/// Returns authorization, parsing, or storage failures.
pub fn legacy_execution_records<B: ApplicationBackend>(
    service: &scriberig_app::ApplicationService<B>,
    context: &ApplicationContext,
    session: &str,
) -> Result<Vec<scriberig_protocol::LegacyExecutionRecordView>, scriberig_app::ApplicationError> {
    let session = parse_session(session)?;
    AgentConsoleBackend::legacy_execution_records(service, context, session).map(|records| {
        records
            .into_iter()
            .map(|record| scriberig_protocol::LegacyExecutionRecordView {
                path: record.path,
                value: record.value,
                target: record
                    .target
                    .map(|target| scriberig_protocol::LegacyExecutionTargetView {
                        plan_id: target.plan_id.to_string(),
                        node_id: target.node_id.to_string(),
                        revision: target.revision,
                        claim: target.claim,
                    }),
            })
            .collect()
    })
}

/// Maps one exact preserved session selection to a current catalog entry.
///
/// - `service`: Shared application service.
/// - `context`: Trusted capability context.
/// - `request`: Exact legacy path, observed revision, and explicit selection.
///
/// Updated session execution selection.
/// # Returns
/// The operation result.
/// # Errors
/// Returns validation, authorization, conflict, or persistence failures.
pub fn map_legacy_execution_selection<B: ApplicationBackend>(
    service: &scriberig_app::ApplicationService<B>,
    context: &ApplicationContext,
    request: &scriberig_protocol::LegacyExecutionMappingRequest,
) -> Result<scriberig_protocol::SessionExecutionSelection, scriberig_app::ApplicationError> {
    let session = parse_session(&request.session.id)?;
    let selection = AgentConsoleBackend::map_legacy_execution_selection(
        service,
        context,
        &scriberig_app::LegacyExecutionMappingRequest {
            session,
            expected_revision: request.expected_revision,
            legacy_path: request.legacy_path.clone(),
            provider_id: request.provider_id.clone(),
            model_id: request.model_id.clone(),
            thinking_level: request.thinking_level.clone(),
        },
    )?;
    Ok(selection_response(selection))
}

/// Maps one current legacy plan-node assignment to an active catalog model.
///
/// - `service`: Shared application service.
/// - `context`: Trusted session and plan mutation context.
/// - `request`: Exact legacy path, plan revision, and operator-selected model.
///
/// Updated protocol plan projection.
/// # Returns
/// The operation result.
/// # Errors
/// Returns validation, authorization, revision, or persistence failures.
pub fn map_legacy_plan_assignment<B: ApplicationBackend>(
    service: &scriberig_app::ApplicationService<B>,
    context: &ApplicationContext,
    request: &scriberig_protocol::LegacyPlanAssignmentMappingRequest,
) -> Result<scriberig_protocol::SessionPlan, scriberig_app::ApplicationError> {
    let session = parse_session(&request.session.id)?;
    let plan = AgentConsoleBackend::map_legacy_plan_assignment(
        service,
        context,
        &scriberig_app::LegacyPlanAssignmentMappingRequest {
            session,
            legacy_path: request.legacy_path.clone(),
            expected_plan_revision: request.expected_plan_revision,
            provider_id: request.provider_id.clone(),
            model_id: request.model_id.clone(),
            thinking_level: request.thinking_level.clone(),
        },
    )?;
    super::super::server_helpers::project_session_plan(plan, None)
}
