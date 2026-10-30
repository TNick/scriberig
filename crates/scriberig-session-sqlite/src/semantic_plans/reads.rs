//! Reads complete and compact views from exact semantic plan snapshots.

use std::collections::BTreeMap;

use scriberig_core::{EntityId, SnapshotId};
use scriberig_session::Plan;
use scriberig_store::{RevisionStore, RootStore, Snapshot, SnapshotStore};

use super::history::plan_revision;
use super::plan_root;
use crate::SqliteSessionStore;

impl SqliteSessionStore {
    /// Loads one plan projection from an exact root snapshot.
    ///
    /// # Parameters
    /// - `_session`: Owning session identity retained for call-site context.
    /// - `plan`: Stable plan identity.
    /// - `snapshot`: Exact universal snapshot to assemble.
    ///
    /// # Returns
    /// The plan projection represented by the exact semantic snapshot.
    ///
    /// # Errors
    /// Returns a diagnostic for missing, invalid, or corrupt semantic objects.
    pub(super) fn load_plan_at_snapshot(
        &self,
        _session: EntityId,
        plan: EntityId,
        snapshot: SnapshotId,
    ) -> Result<Plan, String> {
        let current_semantics = self.read_semantic_snapshot(plan, snapshot)?;
        let mut current = current_semantics
            .clone()
            .into_plan(Vec::new())
            .map_err(|error| format!("plan semantic decode failed: {error:?}"))?;
        let mut history = BTreeMap::new();
        for ancestor in self
            .semantic_store
            .ancestors(snapshot)
            .map_err(|error| format!("plan history read failed: {error:?}"))?
        {
            if ancestor == Snapshot::empty().id() {
                continue;
            }
            let historical = self
                .read_semantic_snapshot(plan, ancestor)?
                .into_plan(Vec::new())
                .map_err(|error| format!("plan history decode failed: {error:?}"))?;
            history
                .entry(historical.revision)
                .or_insert_with(|| plan_revision(&historical));
        }
        history
            .entry(current.revision)
            .or_insert_with(|| plan_revision(&current));
        for metadata in current_semantics
            .history_metadata()
            .map_err(|error| format!("plan history metadata is invalid: {error:?}"))?
        {
            if let Some(record) = history.get_mut(&metadata.mutation_revision) {
                record.reason = metadata.reason;
            }
        }
        current.history = history.into_values().collect();
        Ok(current)
    }

    /// Loads an exact plan revision only when its snapshot belongs to that root.
    ///
    /// # Parameters
    /// - `session`: Owning durable session identity.
    /// - `plan`: Stable plan identity.
    /// - `snapshot`: Exact semantic snapshot to resolve.
    ///
    /// The plan projection or `None` when the snapshot is outside its history.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns a persistence diagnostic when ancestry or semantic data cannot
    /// be read.
    pub(crate) fn load_plan_snapshot(
        &self,
        session: EntityId,
        plan: EntityId,
        snapshot: SnapshotId,
    ) -> Result<Option<Plan>, String> {
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
        self.load_plan_at_snapshot(session, plan, snapshot)
            .map(Some)
    }

    /// Loads a compact plan view without reading detailed semantic records.
    ///
    /// # Parameters
    /// - `session`: Owning durable session identity.
    /// - `plan`: Stable plan identity.
    /// - `snapshot`: Exact semantic snapshot to resolve.
    ///
    /// # Returns
    /// The compact plan view, or `None` when the snapshot is outside its root.
    ///
    /// # Errors
    /// Returns a diagnostic when required plan objects cannot be read or decoded.
    pub(crate) fn load_plan_view_snapshot(
        &self,
        session: EntityId,
        plan: EntityId,
        snapshot: SnapshotId,
    ) -> Result<Option<scriberig_session::PlanView>, String> {
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
        let required_types = [
            scriberig_session::schema::SessionEntityType::Plan.as_str(),
            scriberig_session::schema::SessionEntityType::PlanNode.as_str(),
            scriberig_session::schema::SessionEntityType::PlanMembership.as_str(),
            scriberig_session::schema::SessionEntityType::PlanStructure.as_str(),
        ];
        let mut objects = Vec::new();
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
        let required_entries = entries
            .into_iter()
            .filter(|(_, revision_id)| {
                object_types
                    .get(revision_id)
                    .is_some_and(|object_type| required_types.contains(&object_type.as_str()))
            })
            .collect::<Vec<_>>();
        let required_revision_ids = required_entries
            .iter()
            .map(|(_, revision_id)| *revision_id)
            .collect::<Vec<_>>();
        let revisions = self
            .semantic_store
            .get_many(&required_revision_ids)
            .map_err(|error| format!("plan revision batch read failed: {error:?}"))?;
        for (entity, revision_id) in required_entries {
            let Some(revision) = revisions.get(&revision_id) else {
                return Err(format!("plan revision {revision_id} is missing"));
            };
            let Some(object_type) = object_types.get(&revision_id) else {
                return Err(format!("plan revision {revision_id} is missing"));
            };
            if revision.object_type() != object_type {
                return Err(format!("plan revision {revision_id} has mismatched type"));
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
            .plan_view()
            .map(Some)
            .map_err(|error| format!("compact plan view decode failed: {error:?}"))
    }

    /// Loads semantic objects for one exact plan snapshot.
    ///
    /// # Parameters
    /// - `plan`: Expected plan identity.
    /// - `snapshot`: Exact snapshot containing the plan.
    ///
    /// # Returns
    /// The typed object set stored in the snapshot.
    ///
    /// # Errors
    /// Returns an error when a referenced revision is missing or unreadable.
    fn read_semantic_snapshot(
        &self,
        plan: EntityId,
        snapshot: SnapshotId,
    ) -> Result<scriberig_session::PlanSemanticSnapshot, String> {
        let mut objects = Vec::new();
        for (entity, revision_id) in self
            .semantic_store
            .iter(snapshot)
            .map_err(|error| format!("plan snapshot read failed: {error:?}"))?
        {
            let Some(revision) = self
                .semantic_store
                .get(revision_id)
                .map_err(|error| format!("plan revision read failed: {error:?}"))?
            else {
                return Err(format!("plan revision {revision_id} is missing"));
            };
            objects.push(scriberig_session::PlanSemanticRevision {
                entity,
                object_type: revision.object_type().clone(),
                revision: revision.id(),
                payload: revision.payload().to_vec(),
            });
        }
        Ok(scriberig_session::PlanSemanticSnapshot {
            plan,
            snapshot: Some(snapshot),
            objects,
        })
    }

    /// Loads the plan selected by its current named root.
    ///
    /// # Parameters
    /// - `session`: Owning session identity.
    /// - `plan`: Stable plan identity.
    ///
    /// # Returns
    /// The current plan projection.
    ///
    /// # Errors
    /// Returns a diagnostic when the root is absent or its snapshot is invalid.
    pub(super) fn load_semantic_plan(
        &self,
        session: EntityId,
        plan: EntityId,
    ) -> Result<Plan, String> {
        let root = plan_root(session, plan)?;
        let snapshot = self
            .semantic_store
            .read_root(&root)
            .ok_or_else(|| "plan semantic root is absent".to_owned())?;
        self.load_plan_at_snapshot(session, plan, snapshot)
    }
}
