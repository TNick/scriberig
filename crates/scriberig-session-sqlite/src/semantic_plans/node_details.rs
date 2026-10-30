//! Reads bounded gate and evidence pages from exact plan snapshots.

use scriberig_core::{EntityId, SnapshotId};
use scriberig_store::{RevisionStore, RootStore, SnapshotStore};

use super::plan_root;
use crate::SqliteSessionStore;

impl SqliteSessionStore {
    /// Loads a bounded page of gate and evidence records for one node.
    ///
    /// # Parameters
    /// - `session`: Owning durable session identity.
    /// - `plan`: Stable plan identity.
    /// - `node`: Stable plan-node identity.
    /// - `snapshot`: Exact semantic snapshot to resolve.
    /// - `offset`: Number of matching records to skip per category.
    /// - `limit`: Maximum records to return per category.
    ///
    /// # Returns
    /// The requested details, or `None` outside the plan's retained history.
    ///
    /// # Errors
    /// Returns a diagnostic when detail revisions cannot be read or decoded.
    pub(crate) fn load_plan_node_details_snapshot(
        &self,
        session: EntityId,
        plan: EntityId,
        node: EntityId,
        snapshot: SnapshotId,
        offset: usize,
        limit: usize,
    ) -> Result<Option<scriberig_session::PlanSemanticNodeDetails>, String> {
        let root = plan_root(session, plan)?;
        let current = self
            .semantic_store
            .read_root(&root)
            .ok_or_else(|| "plan semantic root is absent".to_owned())?;
        if snapshot != current
            && !self
                .semantic_store
                .ancestors(current)
                .map_err(|error| format!("plan history read failed: {error:?}"))?
                .contains(&snapshot)
        {
            return Ok(None);
        }
        let selected_types = [
            scriberig_session::schema::SessionEntityType::PlanGate.as_str(),
            scriberig_session::schema::SessionEntityType::PlanEvidence.as_str(),
        ];
        let entries = self
            .semantic_store
            .iter(snapshot)
            .map_err(|error| format!("plan snapshot read failed: {error:?}"))?;
        let revision_ids = entries
            .iter()
            .map(|(_, revision)| *revision)
            .collect::<Vec<_>>();
        let object_types = self
            .semantic_store
            .get_object_types(&revision_ids)
            .map_err(|error| format!("plan revision metadata read failed: {error:?}"))?;
        let selected_entries = entries
            .into_iter()
            .filter(|(_, revision)| {
                object_types
                    .get(revision)
                    .is_some_and(|object_type| selected_types.contains(&object_type.as_str()))
            })
            .collect::<Vec<_>>();
        let selected_revision_ids = selected_entries
            .iter()
            .map(|(_, revision)| *revision)
            .collect::<Vec<_>>();
        let revisions = self
            .semantic_store
            .get_many(&selected_revision_ids)
            .map_err(|error| format!("plan detail read failed: {error:?}"))?;
        let mut objects = Vec::with_capacity(selected_entries.len());
        for (entity, revision_id) in selected_entries {
            let Some(revision) = revisions.get(&revision_id) else {
                return Err(format!("plan detail revision {revision_id} is missing"));
            };
            let Some(object_type) = object_types.get(&revision_id) else {
                return Err(format!("plan detail revision {revision_id} has no type"));
            };
            if revision.object_type() != object_type {
                return Err(format!(
                    "plan detail revision {revision_id} has mismatched type"
                ));
            }
            objects.push(scriberig_session::PlanSemanticRevision {
                entity,
                object_type: object_type.clone(),
                revision: revision.id(),
                payload: revision.payload().to_vec(),
            });
        }
        let semantic = scriberig_session::PlanSemanticSnapshot {
            plan,
            snapshot: Some(snapshot),
            objects,
        };
        semantic
            .node_details(node, offset, limit)
            .map(Some)
            .map_err(|error| format!("plan detail decode failed: {error:?}"))
    }
}
