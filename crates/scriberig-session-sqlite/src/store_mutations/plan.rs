//! Persists plan branches and delegates canonical plan writes to semantic roots.

use rusqlite::params;
use scriberig_core::EntityId;
use scriberig_session::{PlanStoreError, Session};

use crate::{OptionalRow, SqliteSessionStore, branch_projection, payload};

impl SqliteSessionStore {
    /// Atomically replaces a plan and merges new branch records.
    ///
    /// - `session`: Snapshot containing the proposed plan and new branches.
    /// - `plan_id`: Stable plan identity being replaced.
    /// - `expected_revision`: Plan revision observed by the caller.
    ///
    /// # Returns
    /// Unit after the plan and new branches are committed.
    ///
    /// # Errors
    /// Returns a revision conflict or bounded persistence failure.
    pub(super) fn compare_and_swap_plan_inner(
        &self,
        session: &Session,
        plan_id: EntityId,
        expected_revision: u64,
    ) -> Result<(), PlanStoreError> {
        let proposed = session
            .plans
            .iter()
            .find(|plan| plan.id == plan_id)
            .ok_or_else(|| PlanStoreError::Persistence("proposed plan is absent".to_owned()))?;
        self.compare_and_swap_semantic_plan(
            session.id,
            proposed,
            expected_revision,
            |transaction, current_plan| {
                let current_payload = transaction
                    .query_row(
                        "SELECT payload FROM session_records WHERE session_id=?1",
                        params![session.id.as_bytes().as_slice()],
                        |row| row.get::<_, Vec<u8>>(0),
                    )
                    .optional()
                    .map_err(|error| error.to_string())?
                    .ok_or_else(|| "session is not stored".to_owned())?;
                let decoded = payload::decode(&current_payload)?;
                let mut stored = decoded.session;
                branch_projection::initialize_if_missing(
                    transaction,
                    session.id,
                    &stored.branches,
                )?;
                stored.branches = branch_projection::read(transaction, session.id)?;
                let mut changed_branches = std::collections::BTreeSet::new();
                for branch in &session.branches {
                    if stored.branches.iter().any(|known| known.id == branch.id) {
                        continue;
                    }
                    stored.branches.push(branch.clone());
                    changed_branches.insert(branch.id);
                    if let Some(parent_id) = branch.parent
                        && let Some(parent) = stored
                            .branches
                            .iter_mut()
                            .find(|known| known.id == parent_id)
                        && !parent.children.contains(&branch.id)
                    {
                        parent.children.push(branch.id);
                        changed_branches.insert(parent.id);
                    }
                }
                let settled_results = proposed.settlement_result_refs_since(current_plan);
                for (branch_id, result_id) in settled_results {
                    let proposed_branch = session
                        .branches
                        .iter()
                        .find(|branch| branch.id == branch_id)
                        .ok_or_else(|| "settled worker branch is absent".to_owned())?;
                    if proposed_branch.result.as_ref().map(|result| result.id) != Some(result_id)
                        || proposed_branch.result.as_ref().map(|result| result.branch)
                            != Some(branch_id)
                    {
                        return Err("settled worker result does not match its plan".to_owned());
                    }
                    let stored_branch = stored
                        .branches
                        .iter_mut()
                        .find(|branch| branch.id == branch_id)
                        .ok_or_else(|| "settled worker branch is not stored".to_owned())?;
                    *stored_branch = proposed_branch.clone();
                    changed_branches.insert(branch_id);
                }
                for branch_id in changed_branches {
                    let changed = stored
                        .branches
                        .iter()
                        .find(|branch| branch.id == branch_id)
                        .ok_or_else(|| "changed branch projection is absent".to_owned())?;
                    let ordinal = session
                        .branches
                        .iter()
                        .position(|branch| branch.id == branch_id)
                        .or_else(|| {
                            stored
                                .branches
                                .iter()
                                .position(|branch| branch.id == branch_id)
                        })
                        .ok_or_else(|| "changed branch position is absent".to_owned())?;
                    branch_projection::upsert(transaction, session.id, changed, ordinal)?;
                }
                Ok(())
            },
        )
    }
}
