//! Persists plan branches and delegates canonical plan writes to semantic roots.

use rusqlite::params;
use scriberig_core::EntityId;
use scriberig_session::{PlanStoreError, Session};

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
        self.compare_and_swap_semantic_plan(
            session.id,
            proposed,
            expected_revision,
            |transaction| {
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
                let updated_payload = payload::encode(&stored, &decoded.legacy_execution)?;
                let updated = transaction
                    .execute(
                        "UPDATE session_records SET payload=?2 WHERE session_id=?1",
                        params![session.id.as_bytes().as_slice(), updated_payload],
                    )
                    .map_err(|error| error.to_string())?;
                if updated != 1 {
                    return Err("session record disappeared during plan update".to_owned());
                }
                Ok(())
            },
        )
    }
}
