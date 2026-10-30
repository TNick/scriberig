//! Implements validated durable plan application operations.
use crate::{ApplicationError, PlanCreateRequest};
use scriberig_core::EntityId;
/// Validates assignment identity against upstream model state.
///
/// - `upstream_models`: Host-owned model catalog with provider request data.
/// - `assignment`: Upstream provider, model, and thinking identities.
///
/// Returns invalid-request when an identity is unavailable or unsupported.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn validate_assignment(
    upstream_models: &[rpi_ai::Model],
    assignment: &scriberig_session::ModelAssignment,
) -> Result<(), ApplicationError> {
    let thinking = assignment
        .thinking_level
        .as_deref()
        .map(|thinking| {
            rpi_ai::ThinkingLevel::parse_name(thinking)
                .ok_or_else(|| invalid_plan("assigned thinking level is invalid"))
        })
        .transpose()?
        .unwrap_or(rpi_ai::ThinkingLevel::Off);
    rpi_model_runtime::resolve_selection(
        upstream_models,
        &rpi_model_runtime::ModelSelection {
            provider_id: assignment.provider_id.clone(),
            model_id: assignment.model_id.clone(),
            thinking,
        },
    )
    .map_err(|error| invalid_plan(error.to_string()))?;
    Ok(())
}

/// Creates a structured conflict for a stale plan mutation.
///
/// - `plan`: Stable plan identity whose revision changed.
///
/// A conflict identifying the optimistic plan revision boundary.
/// # Returns
/// The operation result.
pub(super) fn revision_conflict(plan: EntityId) -> ApplicationError {
    ApplicationError::Conflict {
        subject: plan.to_string(),
        details: Box::new(crate::ConflictDetails {
            operation: Some("plan revision".to_owned()),
            ..crate::ConflictDetails::default()
        }),
    }
}

/// Rejects malformed proposal nodes and dependency indexes before allocation.
///
/// - `request`: Proposed structured plan.
///
/// Returns invalid-request for empty, duplicate, or out-of-range proposal data.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn validate_proposal(request: &PlanCreateRequest) -> Result<(), ApplicationError> {
    if request.goal.trim().is_empty() || request.nodes.is_empty() {
        return Err(invalid_plan("plan goal and at least one node are required"));
    }
    validate_replan_drafts(&request.nodes)?;
    for (prerequisite, dependent) in &request.dependencies {
        if *prerequisite >= request.nodes.len() || *dependent >= request.nodes.len() {
            return Err(invalid_plan(
                "dependency references an unknown proposal node",
            ));
        }
    }
    Ok(())
}

/// Validates replan node identity and local parent references.
///
/// - `nodes`: Proposed new plan nodes.
///
/// Returns invalid-request for empty, duplicate, or out-of-range proposal data.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn validate_replan_drafts(
    nodes: &[crate::PlanNodeDraft],
) -> Result<(), ApplicationError> {
    if nodes.is_empty() {
        return Err(invalid_plan("at least one new plan node is required"));
    }
    let mut keys = Vec::with_capacity(nodes.len());
    for (index, node) in nodes.iter().enumerate() {
        let key = node.key.as_str();
        if node.key.trim().is_empty()
            || keys.contains(&key)
            || node.title.trim().is_empty()
            || node.purpose.trim().is_empty()
            || node.parent_index.is_some_and(|parent| parent >= index)
        {
            return Err(invalid_plan(
                "plan node identity, text, or parent is invalid",
            ));
        }
        keys.push(key);
    }
    Ok(())
}

/// Creates a stable application validation error for plan proposals.
///
/// - `message`: Safe reason the plan was rejected.
///
/// A structured invalid-request error.
/// # Returns
/// The operation result.
pub(super) fn invalid_plan(message: impl Into<String>) -> ApplicationError {
    ApplicationError::InvalidRequest {
        operation: "plan".to_owned(),
        message: message.into(),
    }
}
