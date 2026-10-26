//! Reads bounded gate and evidence pages from one canonical plan revision.

use std::str::FromStr;

use scriberig_app::{ApplicationContext, ApplicationError, SessionBackend, SessionRequest};
use scriberig_core::{EntityId, SnapshotId};
use scriberig_protocol::{SessionPlanDetailsRequest, SessionPlanDetailsResponse};

/// Reads stable pages of gates and evidence from one observed plan revision.
///
/// # Parameters
/// - `service`: Shared application service owning session authorization.
/// - `context`: Trusted read context.
/// - `request`: Session, plan, expected revision, and page selection.
///
/// # Returns
/// Bounded canonical gate and evidence details tied to one plan revision.
///
/// # Errors
/// Returns authorization, lookup, stale revision, or invalid-identity errors.
pub(super) fn read<B: scriberig_app::ApplicationBackend>(
    service: &scriberig_app::ApplicationService<B>,
    context: &ApplicationContext,
    request: &SessionPlanDetailsRequest,
) -> Result<SessionPlanDetailsResponse, ApplicationError> {
    let session_id = parse_identity(&request.session.id, "session")?;
    let plan_id = parse_identity(&request.plan, "plan")?;
    let (plan, revision, source_snapshot) = if let Some(snapshot) = request.snapshot.as_deref() {
        let snapshot =
            snapshot
                .parse::<SnapshotId>()
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "plan details".to_owned(),
                    message: format!("invalid snapshot identifier: {error}"),
                })?;
        let plan = service.read_plan_at_snapshot(context, session_id, plan_id, snapshot)?;
        let revision = plan.revision;
        let source_snapshot = plan.source_snapshot;
        (plan, revision, source_snapshot)
    } else {
        let (view, _) = service.read_plan_views(context, session_id, plan_id, None)?;
        let plan = match view.snapshot {
            Some(snapshot) => {
                service.read_plan_at_snapshot(context, session_id, plan_id, snapshot)?
            }
            None => SessionBackend::read_session(
                service,
                context,
                &SessionRequest {
                    session: session_id,
                },
            )?
            .plans
            .into_iter()
            .find(|plan| plan.id == plan_id)
            .ok_or_else(|| ApplicationError::NotFound {
                subject: request.plan.clone(),
            })?,
        };
        (plan, view.mutation_revision, view.snapshot)
    };
    if revision != request.expected_revision {
        return Err(ApplicationError::Conflict {
            subject: request.plan.clone(),
            details: Box::new(scriberig_app::ConflictDetails {
                operation: Some("plan detail page revision".to_owned()),
                ..scriberig_app::ConflictDetails::default()
            }),
        });
    }
    let mut gates = plan.gates.iter().collect::<Vec<_>>();
    gates.sort_by_key(|gate| gate.id);
    let mut evidence = plan.evidence.iter().collect::<Vec<_>>();
    evidence.sort_by_key(|item| item.id);
    let limit = page_limit(request.limit);
    let (gates, gate_total, gate_next_offset) = page(&gates, request.gate_offset, limit);
    let (evidence, evidence_total, evidence_next_offset) =
        page(&evidence, request.evidence_offset, limit);
    Ok(SessionPlanDetailsResponse {
        plan: request.plan.clone(),
        revision,
        snapshot: source_snapshot.map(|snapshot| snapshot.to_string()),
        gate_total,
        gate_offset: request.gate_offset,
        gates: gates
            .into_iter()
            .map(crate::server::server_helpers::session_plan::gate_detail)
            .collect(),
        gate_next_offset,
        evidence_total,
        evidence_offset: request.evidence_offset,
        evidence: evidence
            .into_iter()
            .map(crate::server::server_helpers::session_plan::evidence_detail)
            .collect(),
        evidence_next_offset,
    })
}

/// Returns a bounded collection page and its next offset.
///
/// # Parameters
/// - `values`: Stable ordered collection to page.
/// - `offset`: Requested zero-based offset.
/// - `limit`: Validated positive page size.
///
/// # Returns
/// Cloned page values, total count, and a next offset when more remain.
fn page<T: Clone>(values: &[T], offset: u32, limit: u32) -> (Vec<T>, u32, Option<u32>) {
    let total = u32::try_from(values.len()).unwrap_or(u32::MAX);
    let start = usize::try_from(offset)
        .unwrap_or(usize::MAX)
        .min(values.len());
    let end = start
        .saturating_add(usize::try_from(limit).unwrap_or(usize::MAX))
        .min(values.len());
    (
        values[start..end].to_vec(),
        total,
        (end < values.len()).then(|| u32::try_from(end).unwrap_or(u32::MAX)),
    )
}

/// Resolves the default page size and caps caller-supplied limits.
///
/// # Parameters
/// - `requested`: Requested page size, with zero selecting the default.
///
/// # Returns
/// A page size between one and one hundred items.
fn page_limit(requested: u32) -> u32 {
    if requested == 0 {
        25
    } else {
        requested.min(100)
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
        operation: "plan details".to_owned(),
        message: format!("invalid {kind} identifier: {error}"),
    })
}

#[cfg(test)]
mod tests {
    use super::{page, page_limit};

    /// Verifies page limits and offsets remain bounded at collection edges.
    #[test]
    fn detail_pages_are_bounded_and_keep_next_offsets() {
        let values = (0..101).collect::<Vec<_>>();
        let limit = page_limit(500);
        let (first, total, next) = page(&values, 0, limit);
        assert_eq!(first.len(), 100);
        assert_eq!(total, 101);
        assert_eq!(next, Some(100));
        let (last, _, next) = page(&values, 100, page_limit(0));
        assert_eq!(last, vec![100]);
        assert_eq!(next, None);
    }
}
