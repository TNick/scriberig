//! Implements validated durable plan application operations.
use crate::{ApplicationError, PlanCreateRequest};
use scriberig_core::EntityId;
/// Validates an assignment against the existing model library without credentials.
///
/// - `library`: Configured provider-neutral model library.
/// - `assignment`: Safe profile, model, and thinking identities.
///
/// Returns invalid-request when an identity is unavailable or unsupported.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn validate_assignment(
    library: &crate::ModelLibrary,
    assignment: &scriberig_session::ModelAssignment,
) -> Result<(), ApplicationError> {
    let profile = library
        .profile(&assignment.profile)
        .filter(|profile| profile.enabled)
        .ok_or_else(|| invalid_plan("assigned model profile is unavailable"))?;
    let model = library
        .model(&profile.model)
        .ok_or_else(|| invalid_plan("assigned model profile has no model"))?;
    if assignment.model != model.id.as_str() {
        return Err(invalid_plan("assigned model does not match its profile"));
    }
    if assignment
        .thinking_level
        .as_deref()
        .is_some_and(|thinking| {
            !model
                .thinking_levels
                .iter()
                .any(|level| level.id.as_str() == thinking)
        })
    {
        return Err(invalid_plan("assigned thinking level is unsupported"));
    }
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
