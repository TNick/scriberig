//! Persists plan branches and delegates canonical plan writes to semantic roots.

use rusqlite::{TransactionBehavior, params};
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
        self.merge_session_branches(session, plan_id, expected_revision, proposed.revision)?;
        self.compare_and_swap_semantic_plan(session.id, proposed, expected_revision)?;
        if let Ok(Some(updated)) = SessionStore::load(self, session.id) {
            if let Some(plan) = updated.plans.iter().find(|plan| plan.id == plan_id) {
                let _ = self.update_plan_rollup_index_after_cas(session.id, &previous_plan, plan);
            }
        }
        Ok(())
    }

    /// Merges new session branches before the canonical plan root is changed.
    ///
    /// # Parameters
    /// - `self`: SQLite session repository.
    /// - `session`: Caller projection containing new branch records.
    /// - `plan_id`: Plan whose expected mutation is being persisted.
    /// - `expected_revision`: Plan revision observed before the mutation.
    /// - `proposed_revision`: Plan revision proposed by the caller.
    ///
    /// # Returns
    /// Unit after the compatibility session projection transaction commits.
    ///
    /// # Errors
    /// Returns a persistence diagnostic for an invalid revision or SQL failure.
    fn merge_session_branches(
        &self,
        session: &Session,
        plan_id: EntityId,
        expected_revision: u64,
        proposed_revision: u64,
    ) -> Result<(), PlanStoreError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| PlanStoreError::Persistence("session lock poisoned".to_owned()))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| PlanStoreError::Persistence(error.to_string()))?;
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
            .find(|plan| plan.id == plan_id)
            .ok_or_else(|| PlanStoreError::Persistence("plan is not stored".to_owned()))?;
        if proposed_revision
            != expected_revision.checked_add(1).ok_or_else(|| {
                PlanStoreError::Persistence("plan revision is exhausted".to_owned())
            })?
        {
            return Err(PlanStoreError::Persistence(
                "proposed plan has an unexpected revision".to_owned(),
            ));
        }
        for branch in &session.branches {
            if stored.branches.iter().any(|known| known.id == branch.id) {
                continue;
            }
            stored.branches.push(branch.clone());
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
        transaction
            .commit()
            .map_err(|error| PlanStoreError::Persistence(error.to_string()))?;
        drop(connection);
        Ok(())
    }
}
