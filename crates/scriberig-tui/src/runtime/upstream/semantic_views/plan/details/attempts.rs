//! Formats worker attempt provenance in the selected plan-node details.

use std::fmt::Write as _;

use scriberig_core::EntityId;
use scriberig_session::Plan;

/// Appends bounded attempt provenance without exposing branch transcript content.
///
/// - `output`: Markdown receiving safe plan details.
/// - `plan`: Durable plan containing immutable attempt history.
/// - `node`: Stable selected plan-node identity.
pub(super) fn append_node_attempts(output: &mut String, plan: &Plan, node: EntityId) {
    let Some(attempts) = plan.attempt_history(node) else {
        return;
    };
    if attempts.is_empty() {
        return;
    }
    super::super::super::common::append_value(output, "## Worker attempts");
    for attempt in attempts.iter().rev().take(20).rev() {
        let mut detail = format!(
            "Generation {} · {:?} · branch {}",
            attempt.claim_generation, attempt.status, attempt.branch
        );
        if let Some(claim) = attempt.claim {
            let _ = write!(detail, " · claim {claim}");
        }
        if let Some(result) = attempt.result {
            let _ = write!(detail, " · result {result}");
        }
        if let Some(failure) = &attempt.failure {
            let _ = write!(detail, " · failure {}", failure.reference.entity);
        }
        super::super::super::common::append_value(output, &detail);
    }
    if attempts.len() > 20 {
        super::super::super::common::append_value(
            output,
            &format!("{} earlier attempts omitted.", attempts.len() - 20),
        );
    }
}
