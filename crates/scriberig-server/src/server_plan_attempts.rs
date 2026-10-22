//! Reads bounded plan-attempt history through shared session semantics.

use std::str::FromStr;

use scriberig_app::{ApplicationContext, ApplicationError, SessionBackend, SessionRequest};
use scriberig_core::EntityId;
use scriberig_protocol::{SessionPlanAttemptsRequest, SessionPlanAttemptsResponse};

/// Reads one bounded plan-node attempt page from the canonical session.
///
/// # Parameters
/// - `service`: Shared application service owning session authorization.
/// - `context`: Trusted read context.
/// - `request`: Session, plan, node, and page selection.
///
/// # Returns
/// Safe attempt metadata without worker transcript content.
///
/// # Errors
/// Returns authorization, lookup, or invalid-identity errors.
pub(super) fn read<B: scriberig_app::ApplicationBackend>(
    service: &scriberig_app::ApplicationService<B>,
    context: &ApplicationContext,
    request: &SessionPlanAttemptsRequest,
) -> Result<SessionPlanAttemptsResponse, ApplicationError> {
    let session_id = parse_identity(&request.session.id, "session")?;
    let plan_id = parse_identity(&request.plan, "plan")?;
    let node_id = parse_identity(&request.node, "node")?;
    let session = SessionBackend::read_session(
        service,
        context,
        &SessionRequest {
            session: session_id,
        },
    )?;
    let plan = session
        .plans
        .iter()
        .find(|plan| plan.id == plan_id)
        .ok_or_else(|| ApplicationError::NotFound {
            subject: request.plan.clone(),
        })?;
    let attempts = plan
        .attempt_history(node_id)
        .ok_or_else(|| ApplicationError::NotFound {
            subject: request.node.clone(),
        })?;
    Ok(response_page(
        &request.plan,
        &request.node,
        &attempts,
        request.offset,
        request.limit,
    ))
}

/// Builds one capped response page without changing attempt ordering.
///
/// # Parameters
/// - `plan`: Stable plan identity for the response.
/// - `node`: Stable node identity for the response.
/// - `attempts`: Ordered history reconstructed from immutable plan revisions.
/// - `offset`: Requested zero-based history offset.
/// - `requested_limit`: Requested page size, with zero selecting the default.
///
/// # Returns
/// One bounded response page with a continuation offset when needed.
fn response_page(
    plan: &str,
    node: &str,
    attempts: &[scriberig_session::PlanAttemptView],
    offset: u32,
    requested_limit: u32,
) -> SessionPlanAttemptsResponse {
    let total = u32::try_from(attempts.len()).unwrap_or(u32::MAX);
    let limit = if requested_limit == 0 {
        25
    } else {
        requested_limit.min(100)
    };
    let start = usize::try_from(offset)
        .unwrap_or(usize::MAX)
        .min(attempts.len());
    let end = start
        .saturating_add(usize::try_from(limit).unwrap_or(usize::MAX))
        .min(attempts.len());
    SessionPlanAttemptsResponse {
        plan: plan.to_owned(),
        node: node.to_owned(),
        total,
        offset,
        attempts: attempts[start..end]
            .iter()
            .map(crate::server::server_helpers::session_plan::attempt)
            .collect(),
        next_offset: (end < attempts.len()).then(|| u32::try_from(end).unwrap_or(u32::MAX)),
    }
}

/// Parses one stable entity identity or returns a structured request error.
///
/// # Parameters
/// - `value`: Serialized entity identity.
/// - `kind`: Request field name used in the diagnostic.
///
/// # Returns
/// The parsed entity identity.
///
/// # Errors
/// Returns `InvalidRequest` when the identity is malformed.
fn parse_identity(value: &str, kind: &str) -> Result<EntityId, ApplicationError> {
    EntityId::from_str(value).map_err(|error| ApplicationError::InvalidRequest {
        operation: "plan attempts".to_owned(),
        message: format!("invalid {kind} identifier: {error}"),
    })
}

#[cfg(test)]
mod tests {
    use super::response_page;
    use scriberig_session::{PlanAttemptView, PlanNodeStatus};

    /// Creates one ordered failure attempt for paging checks.
    ///
    /// # Parameters
    /// - `generation`: Unique ordered node-local attempt generation.
    ///
    /// # Returns
    /// One transcript-free plan attempt view.
    fn attempt(generation: u64) -> PlanAttemptView {
        let mut branch = [0; 16];
        branch[..8].copy_from_slice(&generation.to_le_bytes());
        PlanAttemptView {
            branch: scriberig_core::EntityId::from_bytes(branch),
            claim_generation: generation,
            claim: None,
            status: PlanNodeStatus::Failed,
            result: None,
            failure: None,
        }
    }

    /// Verifies attempt pages cap their size and preserve generation order.
    #[test]
    fn pages_are_bounded_and_ordered() {
        let attempts = (1..=101).map(attempt).collect::<Vec<_>>();

        let first = response_page("plan", "node", &attempts, 0, 1000);
        assert_eq!(first.total, 101);
        assert_eq!(first.attempts.len(), 100);
        assert_eq!(first.attempts[0].claim_generation, 1);
        assert_eq!(first.attempts[99].claim_generation, 100);
        assert_eq!(first.next_offset, Some(100));

        let default = response_page("plan", "node", &attempts, 0, 0);
        assert_eq!(default.attempts.len(), 25);
        assert_eq!(default.next_offset, Some(25));

        let last = response_page("plan", "node", &attempts, 100, 0);
        assert_eq!(last.attempts.len(), 1);
        assert_eq!(last.attempts[0].claim_generation, 101);
        assert_eq!(last.next_offset, None);
    }
}
