//! Persists plan branches and delegates canonical plan writes to semantic roots.

use rusqlite::{Transaction, params};
use scriberig_core::EntityId;
use scriberig_session::{PlanStoreError, Session, SessionStore};

use crate::{OptionalRow, SqliteSessionStore, payload};

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
        let current_payload = {
            let connection = self
                .connection
                .lock()
                .map_err(|_| PlanStoreError::Persistence("session lock poisoned".to_owned()))?;
            connection
                .query_row(
                    "SELECT payload FROM session_records WHERE session_id=?1",
                    params![session.id.as_bytes().as_slice()],
                    |row| row.get::<_, Vec<u8>>(0),
                )
                .optional()
                .map_err(|error| PlanStoreError::Persistence(error.to_string()))?
                .ok_or_else(|| PlanStoreError::Persistence("session is not stored".to_owned()))?
        };
        let mut current = payload::decode(&current_payload)
            .map_err(PlanStoreError::Persistence)?
            .session;
        self.hydrate_semantic_plans(&mut current)
            .map_err(PlanStoreError::Persistence)?;
        let current_plan = current
            .plans
            .iter()
            .find(|plan| plan.id == plan_id)
            .ok_or_else(|| PlanStoreError::Persistence("plan is not stored".to_owned()))?;
        if current_plan.revision != expected_revision {
            return Err(PlanStoreError::RevisionConflict {
                expected: expected_revision,
                actual: current_plan.revision,
            });
        }
        let previous_plan = current_plan.clone();
        self.compare_and_swap_semantic_plan(
            session.id,
            proposed,
            expected_revision,
            |transaction| {
                Self::merge_session_branches_in_transaction(
                    transaction,
                    session,
                    &previous_plan,
                    proposed,
                    expected_revision,
                )
                .map_err(|error| format!("{error:?}"))
            },
        )?;
        if let Ok(Some(updated)) = SessionStore::load(self, session.id) {
            if let Some(plan) = updated.plans.iter().find(|plan| plan.id == plan_id) {
                let _ = self.update_plan_rollup_index_after_cas(session.id, &previous_plan, plan);
            }
        }
        Ok(())
    }

    /// Merges changed branches in the transaction that promotes the plan root.
    ///
    /// # Parameters
    /// - `transaction`: Active semantic-root promotion transaction.
    /// - `session`: Caller projection containing the affected branch records.
    /// - `previous_plan`: Plan revision currently stored by the repository.
    /// - `proposed_plan`: Plan revision being committed.
    /// - `expected_revision`: Plan revision observed before the mutation.
    ///
    /// # Returns
    /// Unit after the compatibility session projection write is staged.
    ///
    /// # Errors
    /// Returns a persistence diagnostic for an invalid revision or SQL failure;
    /// the outer root-promotion transaction rolls back both writes on error.
    pub(super) fn merge_session_branches_in_transaction(
        transaction: &Transaction<'_>,
        session: &Session,
        previous_plan: &scriberig_session::Plan,
        proposed_plan: &scriberig_session::Plan,
        expected_revision: u64,
    ) -> Result<(), PlanStoreError> {
        let payload = transaction
            .query_row(
                "SELECT payload FROM session_records WHERE session_id=?1",
                params![session.id.as_bytes().as_slice()],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .optional()
            .map_err(|error| PlanStoreError::Persistence(error.to_string()))?
            .ok_or_else(|| PlanStoreError::Persistence("session is not stored".to_owned()))?;
        let decoded = payload::decode(&payload).map_err(PlanStoreError::Persistence)?;
        let legacy_execution = decoded.legacy_execution;
        let mut stored = decoded.session;
        stored
            .plans
            .iter()
            .find(|plan| plan.id == proposed_plan.id)
            .ok_or_else(|| PlanStoreError::Persistence("plan is not stored".to_owned()))?;
        if proposed_plan.revision
            != expected_revision.checked_add(1).ok_or_else(|| {
                PlanStoreError::Persistence("plan revision is exhausted".to_owned())
            })?
        {
            return Err(PlanStoreError::Persistence(
                "proposed plan has an unexpected revision".to_owned(),
            ));
        }

        let changed_worker_branches = proposed_plan
            .nodes
            .iter()
            .filter(|proposed_node| {
                previous_plan
                    .nodes
                    .iter()
                    .find(|previous_node| previous_node.id == proposed_node.id)
                    != Some(*proposed_node)
            })
            .filter_map(|node| node.branch)
            .collect::<std::collections::BTreeSet<_>>();

        for branch in &session.branches {
            if let Some(index) = stored
                .branches
                .iter()
                .position(|known| known.id == branch.id)
            {
                if changed_worker_branches.contains(&branch.id) {
                    let mut updated = branch.clone();
                    for child in &stored.branches[index].children {
                        if !updated.children.contains(child) {
                            updated.children.push(*child);
                        }
                    }
                    stored.branches[index] = updated;
                }
            } else {
                stored.branches.push(branch.clone());
            }
            if let Some(parent_id) = branch.parent
                && let Some(parent) = stored
                    .branches
                    .iter_mut()
                    .find(|known| known.id == parent_id)
                && !parent.children.contains(&branch.id)
            {
                parent.children.push(branch.id);
            }
        }
        let payload =
            payload::encode(&stored, &legacy_execution).map_err(PlanStoreError::Persistence)?;
        transaction
            .execute(
                "UPDATE session_records SET payload=?2 WHERE session_id=?1",
                params![session.id.as_bytes().as_slice(), payload],
            )
            .map_err(|error| PlanStoreError::Persistence(error.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    //! Verifies plan-root and session-branch writes share one transaction.

    use super::SqliteSessionStore;
    use scriberig_core::EntityId;
    use scriberig_session::{
        Branch, Plan, PlanStatus, Session, SessionExecutionSelection, SessionStore,
    };

    /// Builds a valid running plan and its owning root branch.
    ///
    /// - `session_id`: Stable owning session identity.
    /// - `plan_id`: Stable plan identity.
    /// - `root_branch`: Existing branch that owns the plan.
    ///
    /// # Returns
    /// A session containing the running plan and its root branch.
    fn running_session(session_id: EntityId, plan_id: EntityId, root_branch: EntityId) -> Session {
        let mut plan = Plan::new(
            plan_id,
            session_id,
            scriberig_session::ContinuationAnchor::BranchHead {
                branch: root_branch,
            },
            "atomic settlement",
        )
        .expect("create plan");
        plan.transition(PlanStatus::Planned).expect("plan plan");
        plan.transition(PlanStatus::Running).expect("start plan");
        Session {
            id: session_id,
            branches: vec![Branch::new(root_branch, "root").expect("root branch")],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![plan],
            execution: SessionExecutionSelection::default(),
        }
    }

    /// Rolls back branch projection changes when semantic root promotion fails.
    #[test]
    fn root_promotion_failure_rolls_back_session_branch_projection() {
        let session_id = EntityId::from_bytes([81; 16]);
        let root_branch = EntityId::from_bytes([82; 16]);
        let worker_branch = EntityId::from_bytes([83; 16]);
        let plan_id = EntityId::from_bytes([84; 16]);
        let store = SqliteSessionStore::open_in_memory().expect("open store");
        store
            .save(&running_session(session_id, plan_id, root_branch))
            .expect("save initial session");
        let mut proposed = store
            .load(session_id)
            .expect("load initial session")
            .expect("stored session");
        let expected_revision = proposed.plans[0].revision;
        proposed.plans[0]
            .transition(PlanStatus::Paused)
            .expect("pause plan");
        proposed.branches.push(
            Branch::new_child(worker_branch, root_branch, "worker", "work").expect("worker branch"),
        );
        {
            let connection = store.connection.lock().expect("session connection");
            connection
                .execute_batch(
                    "CREATE TRIGGER reject_plan_root_update
                     BEFORE UPDATE ON roots
                     BEGIN SELECT RAISE(ABORT, 'injected root promotion failure'); END;",
                )
                .expect("install root failure trigger");
        }

        assert!(
            store
                .compare_and_swap_plan(&proposed, plan_id, expected_revision)
                .is_err()
        );

        let reloaded = store
            .load(session_id)
            .expect("reload after rejected promotion")
            .expect("stored session");
        assert_eq!(reloaded.plans[0].revision, expected_revision);
        assert_eq!(reloaded.plans[0].status, PlanStatus::Running);
        assert!(
            !reloaded
                .branches
                .iter()
                .any(|branch| branch.id == worker_branch)
        );
    }

    /// Rolls back semantic root changes when session projection update fails.
    #[test]
    fn session_projection_failure_rolls_back_semantic_root_promotion() {
        let session_id = EntityId::from_bytes([91; 16]);
        let root_branch = EntityId::from_bytes([92; 16]);
        let worker_branch = EntityId::from_bytes([93; 16]);
        let plan_id = EntityId::from_bytes([94; 16]);
        let store = SqliteSessionStore::open_in_memory().expect("open store");
        store
            .save(&running_session(session_id, plan_id, root_branch))
            .expect("save initial session");
        let mut proposed = store
            .load(session_id)
            .expect("load initial session")
            .expect("stored session");
        let expected_revision = proposed.plans[0].revision;
        proposed.plans[0]
            .transition(PlanStatus::Paused)
            .expect("pause plan");
        proposed.branches.push(
            Branch::new_child(worker_branch, root_branch, "worker", "work").expect("worker branch"),
        );
        {
            let connection = store.connection.lock().expect("session connection");
            connection
                .execute_batch(
                    "CREATE TRIGGER reject_session_projection_update
                     BEFORE UPDATE ON session_records
                     BEGIN SELECT RAISE(ABORT, 'injected session projection failure'); END;",
                )
                .expect("install session failure trigger");
        }

        assert!(
            store
                .compare_and_swap_plan(&proposed, plan_id, expected_revision)
                .is_err()
        );

        let reloaded = store
            .load(session_id)
            .expect("reload after rejected projection")
            .expect("stored session");
        assert_eq!(reloaded.plans[0].revision, expected_revision);
        assert_eq!(reloaded.plans[0].status, PlanStatus::Running);
        assert!(
            !reloaded
                .branches
                .iter()
                .any(|branch| branch.id == worker_branch)
        );
    }
}
