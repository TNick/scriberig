//! Implements application-owned legacy execution selection recovery.

use super::super::service_sessions::{
    authorize_session_model_selection, authorize_session_mutation,
};
use super::{ensure_revision, record_selection_change, selection_error};
use crate::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationService,
    LegacyExecutionMappingRequest, LegacyPlanAssignmentMappingRequest,
};
use scriberig_core::EntityId;
use scriberig_session::{
    AgentActivityKind, LegacyExecutionTarget, ModelAssignment, SelectionError, SelectionStoreError,
    SessionExecutionSelection,
};

/// Lists sanitized legacy choices retained for explicit operator recovery.
///
/// - `service`: Application service owning the durable session repository.
/// - `context`: Authorized application request context.
/// - `session`: Stable session identity to inspect.
///
/// # Returns
/// Preserved legacy fields and exact source paths.
///
/// # Errors
/// Returns authorization, unsupported-operation, or persistence errors.
pub(super) fn legacy_execution_records<B: ApplicationBackend>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    session: EntityId,
) -> Result<Vec<scriberig_session::LegacyExecutionRecord>, ApplicationError> {
    authorize_session_model_selection(context, session)?;
    service
        .session_store
        .as_ref()
        .ok_or_else(|| ApplicationError::UnsupportedOperation {
            operation: "legacy execution recovery inspection".to_owned(),
        })?
        .legacy_execution_records(session)
        .map_err(|message| ApplicationError::Internal { message })
}

/// Maps one legacy session choice to an exact selection from the active upstream catalog.
///
/// - `service`: Application service owning catalog, persistence, and operation state.
/// - `context`: Authorized application request context.
/// - `request`: Exact legacy path, revision, and operator-selected upstream pair.
///
/// # Returns
/// The persisted selection after its matching recovery record is removed.
///
/// # Errors
/// Returns authorization, invalid selection, revision, recovery, or persistence errors.
pub(super) fn map_legacy_execution_selection<B: ApplicationBackend>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    request: &LegacyExecutionMappingRequest,
) -> Result<SessionExecutionSelection, ApplicationError> {
    authorize_session_model_selection(context, request.session)?;
    let store =
        service
            .session_store
            .as_ref()
            .ok_or_else(|| ApplicationError::UnsupportedOperation {
                operation: "legacy execution recovery".to_owned(),
            })?;
    let records = store
        .legacy_execution_records(request.session)
        .map_err(|message| ApplicationError::Internal { message })?;
    let _record = records
        .iter()
        .find(|record| record.path == request.legacy_path)
        .ok_or_else(|| ApplicationError::NotFound {
            subject: format!("legacy execution record {}", request.legacy_path),
        })?;
    if request.legacy_path != "/execution" {
        return Err(ApplicationError::InvalidRequest {
            operation: "legacy session selection mapping".to_owned(),
            message: "session selection recovery requires the /execution record".to_owned(),
        });
    }
    let mut session = store
        .load(request.session)
        .map_err(|message| ApplicationError::Internal { message })?
        .ok_or_else(|| ApplicationError::NotFound {
            subject: format!("session {}", request.session),
        })?;
    ensure_revision(
        request.expected_revision,
        session.execution.revision,
        request.session,
    )?;
    let (model, effective_thinking) = service.resolve_upstream_model(
        &request.provider_id,
        &request.model_id,
        request.thinking_level.as_deref(),
    )?;
    let thinking = request
        .thinking_level
        .as_ref()
        .map(|_| effective_thinking.as_str().to_owned());
    session
        .execution
        .replace(
            request.expected_revision,
            Some(model.provider.clone()),
            Some(model.id.clone()),
            thinking,
        )
        .map_err(|error| selection_error(error, request.session))?;
    let summary = format!("Legacy model choice mapped to: {}", model.name);
    record_selection_change(
        &mut session.execution,
        AgentActivityKind::ModelSelectionChanged,
        summary.clone(),
    );
    let selection = session.execution.clone();
    store
        .resolve_legacy_execution_selection(
            &session,
            request.expected_revision,
            &request.legacy_path,
        )
        .map_err(|error| match error {
            SelectionStoreError::RevisionConflict { expected, actual } => selection_error(
                SelectionError::RevisionConflict { expected, actual },
                request.session,
            ),
            SelectionStoreError::LegacyRecordNotFound => ApplicationError::NotFound {
                subject: format!("legacy execution record {}", request.legacy_path),
            },
            SelectionStoreError::Persistence(message) => ApplicationError::Internal { message },
        })?;
    service.operations.publish_session_selection_changed(
        request.session,
        AgentActivityKind::ModelSelectionChanged,
        &summary,
    );
    service.operations.publish_session_changed(request.session);
    Ok(selection)
}

/// Maps a current legacy plan-node assignment through the active catalog and plan CAS.
///
/// - `service`: Application service owning plan and model catalog state.
/// - `context`: Authorized session and plan mutation context.
/// - `request`: Exact legacy path, observed plan revision, and model choice.
///
/// The updated plan after its recovery record is removed.
/// # Returns
/// The operation result.
/// # Errors
/// Returns authorization, catalog, revision, node-state, or persistence errors.
pub(super) fn map_legacy_plan_assignment<B: ApplicationBackend>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    request: &LegacyPlanAssignmentMappingRequest,
) -> Result<scriberig_session::Plan, ApplicationError> {
    authorize_session_mutation(context, request.session)?;
    let store =
        service
            .session_store
            .as_ref()
            .ok_or_else(|| ApplicationError::UnsupportedOperation {
                operation: "legacy plan assignment recovery".to_owned(),
            })?;
    let records = store
        .legacy_execution_records(request.session)
        .map_err(|message| ApplicationError::Internal { message })?;
    let record = records
        .iter()
        .find(|record| record.path == request.legacy_path)
        .ok_or_else(|| ApplicationError::NotFound {
            subject: format!("legacy execution record {}", request.legacy_path),
        })?;
    let target = record
        .target
        .as_ref()
        .filter(|target| !target.claim)
        .ok_or_else(|| ApplicationError::InvalidRequest {
            operation: "legacy plan assignment mapping".to_owned(),
            message: "worker-claim assignments cannot be mapped".to_owned(),
        })?;
    map_plan_assignment(service, context, request, target)
}

/// Maps a legacy plan assignment through the active catalog and current plan CAS.
///
/// - `service`: Application service owning plan and model catalog state.
/// - `context`: Authorized session and plan mutation context.
/// - `request`: Exact record path and operator-selected upstream pair.
/// - `target`: Stable plan and node identity captured during legacy import.
/// - `expected_revision`: Plan revision observed by the operator.
///
/// The updated plan after its matching recovery record is removed.
/// # Returns
/// The operation result.
/// # Errors
/// Returns authorization, catalog, revision, node-state, or persistence errors.
fn map_plan_assignment<B: ApplicationBackend>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    request: &LegacyPlanAssignmentMappingRequest,
    target: &LegacyExecutionTarget,
) -> Result<scriberig_session::Plan, ApplicationError> {
    authorize_session_mutation(context, request.session)?;
    let store =
        service
            .session_store
            .as_ref()
            .ok_or_else(|| ApplicationError::UnsupportedOperation {
                operation: "legacy plan assignment recovery".to_owned(),
            })?;
    let (model, effective_thinking) = service.resolve_upstream_model(
        &request.provider_id,
        &request.model_id,
        request.thinking_level.as_deref(),
    )?;
    let assignment = ModelAssignment {
        provider_id: model.provider.clone(),
        model_id: model.id.clone(),
        thinking_level: request
            .thinking_level
            .as_ref()
            .map(|_| effective_thinking.as_str().to_owned()),
    };
    let updated = crate::SessionBackend::set_plan_node_assignment(
        service,
        context,
        &crate::PlanAssignmentRequest {
            session: request.session,
            plan: target.plan_id,
            node: target.node_id,
            expected_revision: request.expected_plan_revision,
            assignment: Some(assignment),
        },
    )?;
    store
        .resolve_legacy_plan_assignment(request.session, &request.legacy_path, target)
        .map_err(map_recovery_store_error)?;
    Ok(updated)
}

/// Converts recovery-store errors into application errors.
///
/// - `error`: Typed storage failure.
///
/// Structured application error safe for the caller.
/// # Returns
/// The mapped application error.
fn map_recovery_store_error(error: SelectionStoreError) -> ApplicationError {
    match error {
        SelectionStoreError::LegacyRecordNotFound => ApplicationError::NotFound {
            subject: "legacy plan assignment record".to_owned(),
        },
        SelectionStoreError::RevisionConflict { expected, actual } => ApplicationError::Conflict {
            subject: format!("legacy plan assignment revision {expected}/{actual}"),
            details: Box::new(crate::ConflictDetails::default()),
        },
        SelectionStoreError::Persistence(message) => ApplicationError::Internal { message },
    }
}
