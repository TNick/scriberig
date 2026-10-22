//! Persists plan revisions through the universal immutable object store.

#[path = "semantic_plans/history.rs"]
mod history;
use history::{plan_at_revision, plan_revision, semantic_writes};
#[path = "semantic_plans/equivalence.rs"]
mod equivalence;
#[path = "semantic_plans/projection.rs"]
mod projection;

use std::collections::BTreeMap;

use scriberig_core::{EncodingVersion, EntityId, RootName, SnapshotId};
use scriberig_session::{Plan, PlanStoreError, Session};
use scriberig_store::{
    Revision, RevisionStore, RootStore, Snapshot, SnapshotChange, SnapshotStore,
};
use scriberig_store_sqlite::PromotionError;

use crate::SqliteSessionStore;

impl SqliteSessionStore {
    /// Replaces aggregate plan projections with the universal-store revisions.
    ///
    /// Existing aggregate plans are imported once when their semantic root is
    /// absent. Later reads always resolve the current plan from that root.
    ///
    /// # Parameters
    /// - `session`: Session projection whose plans are hydrated.
    ///
    /// # Errors
    /// Returns a persistence diagnostic when migration or semantic loading
    /// fails.
    ///
    /// # Returns
    /// The value described by the return type.
    pub(super) fn hydrate_semantic_plans(&self, session: &mut Session) -> Result<(), String> {
        let mut plans = Vec::with_capacity(session.plans.len());
        for legacy_plan in &session.plans {
            self.migrate_plan_history(session.id, legacy_plan)?;
            plans.push(self.load_semantic_plan(session.id, legacy_plan.id)?);
        }
        session.plans = plans;
        Ok(())
    }

    /// Builds a compatibility projection without accepting aggregate updates.
    ///
    /// # Parameters
    /// - `proposed`: Caller session projection to persist.
    /// - `prior`: Previously persisted session projection, when available.
    ///
    /// # Returns
    /// A projection whose plans are loaded from semantic roots, with prior
    /// plans retained if the caller supplied a stale session snapshot.
    ///
    /// # Errors
    /// Returns a persistence diagnostic when plan initialization or loading
    /// fails.
    pub(super) fn prepare_session_projection(
        &self,
        proposed: &Session,
        prior: Option<crate::payload::DecodedSession>,
    ) -> Result<Session, String> {
        let mut projection = proposed.clone();
        if let Some(prior) = prior {
            for old_plan in prior.session.plans {
                let root = plan_root(proposed.id, old_plan.id)?;
                if self.semantic_store.read_root(&root).is_none() {
                    self.migrate_plan_history(proposed.id, &old_plan)?;
                }
                if !projection.plans.iter().any(|plan| plan.id == old_plan.id) {
                    projection
                        .plans
                        .push(self.load_semantic_plan(proposed.id, old_plan.id)?);
                }
            }
        }
        self.hydrate_semantic_plans(&mut projection)?;
        Ok(projection)
    }

    /// Promotes one plan mutation through a universal root compare-and-swap.
    ///
    /// # Parameters
    /// - `session`: Owning session identity.
    /// - `proposed`: New plan state to promote.
    /// - `expected_revision`: Plan-local revision observed by the caller.
    ///
    /// # Errors
    /// Returns a plan revision conflict or bounded persistence diagnostic.
    ///
    /// # Returns
    /// The value described by the return type.
    pub(super) fn compare_and_swap_semantic_plan(
        &self,
        session: EntityId,
        proposed: &Plan,
        expected_revision: u64,
    ) -> Result<(), PlanStoreError> {
        let root = plan_root(session, proposed.id).map_err(plan_persistence)?;
        if self.semantic_store.read_root(&root).is_none() {
            self.migrate_plan_history(session, proposed)
                .map_err(plan_persistence)?;
        }
        let current_snapshot = self
            .semantic_store
            .read_root(&root)
            .ok_or_else(|| plan_persistence("plan semantic root disappeared"))?;
        let current = self
            .load_plan_at_snapshot(session, proposed.id, current_snapshot)
            .map_err(plan_persistence)?;
        if current.revision != expected_revision {
            return Err(PlanStoreError::RevisionConflict {
                expected: expected_revision,
                actual: current.revision,
            });
        }
        let next_revision = expected_revision
            .checked_add(1)
            .ok_or_else(|| plan_persistence("plan revision is exhausted"))?;
        if proposed.revision != next_revision {
            return Err(plan_persistence("proposed plan has an unexpected revision"));
        }
        let encoded = scriberig_session::PlanSemanticSnapshot::from_plan(proposed)
            .map_err(|error| plan_persistence(format!("plan encoding failed: {error:?}")))?;
        let old_entries = self
            .semantic_store
            .iter(current_snapshot)
            .map_err(|error| plan_persistence(format!("plan snapshot read failed: {error:?}")))?;
        let new_entities = encoded
            .objects
            .iter()
            .map(|object| object.entity)
            .collect::<std::collections::BTreeSet<_>>();
        let mut revisions = Vec::with_capacity(encoded.objects.len());
        let mut changes = Vec::with_capacity(encoded.objects.len() + old_entries.len());
        for object in encoded.objects {
            let revision = Revision::from_parts(
                object.revision,
                EncodingVersion::V1,
                object.object_type,
                object.payload,
            )
            .map_err(|error| plan_persistence(format!("invalid plan revision: {error:?}")))?;
            changes.push(SnapshotChange::Set(object.entity, revision.id()));
            revisions.push(revision);
        }
        for (entity, _) in old_entries {
            if !new_entities.contains(&entity) {
                changes.push(SnapshotChange::Remove(entity));
            }
        }
        match self.semantic_store.apply_revisions_and_promote(
            root,
            Some(current_snapshot),
            current_snapshot,
            &revisions,
            &changes,
        ) {
            Ok(_) => Ok(()),
            Err(PromotionError::Root(scriberig_store::RootError::Conflict { actual, .. })) => {
                let actual_revision = actual
                    .map(|snapshot| self.load_plan_at_snapshot(session, proposed.id, snapshot))
                    .transpose()
                    .map_err(plan_persistence)?
                    .map_or(current.revision, |plan| plan.revision);
                Err(PlanStoreError::RevisionConflict {
                    expected: expected_revision,
                    actual: actual_revision,
                })
            }
            Err(error) => Err(plan_persistence(format!(
                "plan semantic promotion failed: {error:?}"
            ))),
        }
    }

    /// Imports missing aggregate revisions as a linear semantic snapshot chain.
    ///
    /// # Parameters
    /// - `session`: Owning session identity.
    /// - `plan`: Legacy or newly authored plan to migrate.
    ///
    /// # Errors
    /// Returns a persistence diagnostic when a universal root promotion fails.
    ///
    /// # Returns
    /// The value described by the return type.
    fn migrate_plan_history(&self, session: EntityId, plan: &Plan) -> Result<(), String> {
        let root = plan_root(session, plan.id)?;
        let mut expected = self.semantic_store.read_root(&root);
        let mut base = expected.unwrap_or_else(|| Snapshot::empty().id());
        let current_revision = expected
            .map(|snapshot| self.load_plan_at_snapshot(session, plan.id, snapshot))
            .transpose()?
            .map(|stored| stored.revision);
        let mut history = plan.history.clone();
        history.sort_by_key(|record| record.revision);
        history.dedup_by_key(|record| record.revision);
        if history
            .last()
            .is_none_or(|record| record.revision != plan.revision)
        {
            history.push(plan_revision(plan));
        }
        for record in history {
            if current_revision.is_some_and(|revision| record.revision <= revision) {
                continue;
            }
            let mut historical = if record.revision == plan.revision {
                plan.clone()
            } else {
                plan_at_revision(plan, &record)
            };
            historical.history = plan
                .history
                .iter()
                .filter(|known| known.revision <= record.revision)
                .cloned()
                .collect();
            if !historical
                .history
                .iter()
                .any(|known| known.revision == historical.revision)
            {
                historical.history.push(record);
            }
            let encoded = scriberig_session::PlanSemanticSnapshot::from_plan(&historical)
                .map_err(|error| format!("plan encoding failed: {error:?}"))?;
            equivalence::verify_before_promotion(&historical, &encoded)?;
            let (revisions, changes) = semantic_writes(encoded.objects)?;
            let promoted = self
                .semantic_store
                .apply_revisions_and_promote(root.clone(), expected, base, &revisions, &changes)
                .map_err(|error| format!("plan semantic migration failed: {error:?}"))?;
            base = promoted;
            expected = Some(promoted);
        }
        Ok(())
    }

    /// Loads one plan projection from an exact root snapshot.
    ///
    /// # Parameters
    /// - `session`: Owning session identity.
    /// - `plan`: Stable plan identity.
    /// - `snapshot`: Exact universal snapshot to assemble.
    ///
    /// # Returns
    /// The plan projection represented by the exact semantic snapshot.
    ///
    /// # Errors
    /// Returns a diagnostic for missing, invalid, or corrupt semantic objects.
    /// - `_session`: Owning session identity for the requested snapshot.
    pub(crate) fn load_plan_at_snapshot(
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
    fn load_semantic_plan(&self, session: EntityId, plan: EntityId) -> Result<Plan, String> {
        let root = plan_root(session, plan)?;
        let snapshot = self
            .semantic_store
            .read_root(&root)
            .ok_or_else(|| "plan semantic root is absent".to_owned())?;
        self.load_plan_at_snapshot(session, plan, snapshot)
    }
}

/// Names the canonical universal root for one session plan.
///
/// # Parameters
/// - `session`: Owning session identity.
/// - `plan`: Stable plan identity.
///
/// The deterministic plan root name.
///
/// # Errors
/// Returns an error if the generated name is invalid.
///
/// # Returns
/// The value described by the return type.
pub(crate) fn plan_root(session: EntityId, plan: EntityId) -> Result<RootName, String> {
    RootName::new(format!("scriberig-plan-{session}-{plan}"))
        .map_err(|error| format!("invalid plan root name: {error:?}"))
}

/// Converts a storage diagnostic to the plan repository error type.
///
/// # Parameters
/// - `message`: Bounded persistence diagnostic.
///
/// A typed plan persistence error.
///
/// # Returns
/// The value described by the return type.
fn plan_persistence(message: impl Into<String>) -> PlanStoreError {
    PlanStoreError::Persistence(message.into())
}
