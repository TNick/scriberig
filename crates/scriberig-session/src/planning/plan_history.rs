//! Derives retry and attempt views from immutable plan revisions.

use std::collections::BTreeMap;

use scriberig_core::EntityId;

use super::{Plan, PlanAttemptView, PlanRevision};

impl Plan {
    /// Reconstructs all recorded worker attempts for one stable plan node.
    ///
    /// Failed and completed attempts remain visible after a retry because each
    /// attempt has its own branch and claim generation in immutable history.
    ///
    /// # Parameters
    /// - `self`: Plan containing current and historical revisions.
    /// - `node`: Stable PlanNode identity.
    ///
    /// # Returns
    /// Attempts ordered by claim generation, or `None` if the node is unknown.
    #[must_use]
    pub fn attempt_history(&self, node: EntityId) -> Option<Vec<PlanAttemptView>> {
        let mut snapshots = self.history.iter().collect::<Vec<_>>();
        snapshots.sort_by_key(|revision| revision.revision);
        let known = snapshots
            .iter()
            .any(|revision| revision.nodes.iter().any(|candidate| candidate.id == node))
            || self.nodes.iter().any(|candidate| candidate.id == node);
        if !known {
            return None;
        }
        let current = current_revision(self);
        if snapshots
            .last()
            .is_none_or(|revision| revision.revision != self.revision)
        {
            snapshots.push(&current);
        }
        let mut attempts = BTreeMap::new();
        for revision in snapshots {
            let Some(state) = revision.nodes.iter().find(|candidate| candidate.id == node) else {
                continue;
            };
            let Some(branch) = state.branch else {
                continue;
            };
            let attempt =
                attempts
                    .entry((state.claim_generation, branch))
                    .or_insert(PlanAttemptView {
                        branch,
                        claim_generation: state.claim_generation,
                        claim: None,
                        status: state.status,
                        result: None,
                    });
            if let Some(claim) = state.claim.as_ref() {
                attempt.claim = Some(claim.id);
            }
            attempt.status = state.status;
            attempt.result = state.result;
        }
        Some(attempts.into_values().collect())
    }
}

/// Creates a borrowed-compatible revision snapshot of the current plan state.
///
/// # Parameters
/// - `plan`: Current plan state to include in the history walk.
///
/// The equivalent plan revision record.
fn current_revision(plan: &Plan) -> PlanRevision {
    PlanRevision {
        revision: plan.revision,
        goal: plan.goal.clone(),
        status: plan.status,
        nodes: plan.nodes.clone(),
        dependencies: plan.dependencies.clone(),
        reason: None,
    }
}
