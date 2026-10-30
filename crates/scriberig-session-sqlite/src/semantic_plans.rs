//! Persists plan revisions through the universal immutable object store.

#[path = "semantic_plans/history.rs"]
mod history;
#[path = "semantic_plans/migration_marker.rs"]
mod migration_marker;
#[path = "semantic_plans/node_details.rs"]
mod node_details;
#[path = "semantic_plans/node_index.rs"]
mod node_index;
#[path = "semantic_plans/plan_write_set.rs"]
mod plan_write_set;
#[path = "semantic_plans/reads.rs"]
mod reads;
#[path = "semantic_plans/rollup_index.rs"]
mod rollup_index;
#[path = "semantic_plans/rollup_update.rs"]
mod rollup_update;
use history::{plan_at_revision, plan_revision, semantic_writes};
use plan_write_set::build_plan_write_set;

use scriberig_core::{EntityId, RootName};
use scriberig_session::{Plan, PlanStoreError, Session};
use scriberig_store::{RootStore, Snapshot};
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
    /// - `recover_legacy_claims`: Whether claims imported from legacy data are stale.
    ///
    /// # Errors
    /// Returns a persistence diagnostic when migration or semantic loading
    /// fails.
    /// # Returns
    /// Unit after all plan projections are hydrated.
    pub(super) fn hydrate_semantic_plans(
        &self,
        session: &mut Session,
        recover_legacy_claims: bool,
    ) -> Result<(), String> {
        let mut plans = Vec::with_capacity(session.plans.len());
        for legacy_plan in &session.plans {
            self.migrate_plan_history(session.id, legacy_plan, recover_legacy_claims)?;
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
    /// # Returns
    /// The session projection with canonical plans loaded.
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
                    self.migrate_plan_history(proposed.id, &old_plan, false)?;
                }
                if !projection.plans.iter().any(|plan| plan.id == old_plan.id) {
                    projection
                        .plans
                        .push(self.load_semantic_plan(proposed.id, old_plan.id)?);
                }
            }
        }
        self.hydrate_semantic_plans(&mut projection, false)?;
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
    /// # Returns
    /// Unit after the semantic root is promoted.
    pub(super) fn compare_and_swap_semantic_plan<F>(
        &self,
        session: EntityId,
        proposed: &Plan,
        expected_revision: u64,
        commit_related: F,
    ) -> Result<(), PlanStoreError>
    where
        F: FnOnce(&rusqlite::Transaction<'_>, &Plan) -> Result<(), String>,
    {
        let root = plan_root(session, proposed.id).map_err(plan_persistence)?;
        if self.semantic_store.read_root(&root).is_none() {
            self.migrate_plan_history(session, proposed, false)
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
        let write_set =
            build_plan_write_set(self, current_snapshot, proposed).map_err(plan_persistence)?;
        let staged_rollups = rollup_update::stage_rollup_update(
            self,
            session,
            &current,
            proposed,
            current_snapshot,
            write_set.successor_snapshot,
        )
        .map_err(plan_persistence)?;
        let staged_nodes = node_index::stage_update(
            self,
            session,
            &current,
            proposed,
            current_snapshot,
            write_set.successor_snapshot,
            write_set.indexed_nodes.clone(),
        )
        .map_err(plan_persistence)?;
        match self.semantic_store.apply_revisions_and_promote_with(
            root,
            Some(current_snapshot),
            current_snapshot,
            &write_set.revisions,
            &write_set.changes,
            |transaction| {
                commit_related(transaction, &current)?;
                node_index::persist_in_transaction(
                    transaction,
                    session,
                    proposed.id,
                    &staged_nodes,
                )?;
                rollup_update::persist_in_transaction(
                    transaction,
                    session,
                    proposed.id,
                    &staged_rollups,
                )
            },
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
    /// - `recover_legacy_claims`: Whether imported claims are stale by policy.
    ///
    /// # Errors
    /// Returns a persistence diagnostic when a universal root promotion fails.
    /// # Returns
    /// Unit after all legacy revisions are imported.
    fn migrate_plan_history(
        &self,
        session: EntityId,
        plan: &Plan,
        recover_legacy_claims: bool,
    ) -> Result<(), String> {
        let root = plan_root(session, plan.id)?;
        let mut expected = self.semantic_store.read_root(&root);
        let mut base = expected.unwrap_or_else(|| Snapshot::empty().id());
        let current_revision = expected
            .map(|snapshot| self.load_plan_at_snapshot(session, plan.id, snapshot))
            .transpose()?
            .map(|stored| stored.revision);
        let mut recovery_pending = self.legacy_migration_pending(session, plan.id)?;
        let mut rebuild_indexes = recovery_pending;
        if recover_legacy_claims && expected.is_none() {
            self.set_legacy_migration_pending(session, plan.id, true)?;
            recovery_pending = true;
        }
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
            let (revisions, changes) = semantic_writes(encoded.objects)?;
            let promoted = self
                .semantic_store
                .apply_revisions_and_promote(root.clone(), expected, base, &revisions, &changes)
                .map_err(|error| format!("plan semantic migration failed: {error:?}"))?;
            base = promoted;
            expected = Some(promoted);
            rebuild_indexes = true;
        }
        if recovery_pending
            && (plan.coordinator_claim.is_some()
                || plan.nodes.iter().any(|node| {
                    node.status == scriberig_session::PlanNodeStatus::Running
                        && node.claim.is_some()
                }))
        {
            let mut recovered = plan.clone();
            let count = recovered
                .recover_stale_claims(|_| false)
                .map_err(|error| format!("legacy plan claim recovery failed: {error:?}"))?;
            if count > 0 && current_revision.is_none_or(|revision| revision < recovered.revision) {
                let encoded = scriberig_session::PlanSemanticSnapshot::from_plan(&recovered)
                    .map_err(|error| format!("recovered plan encoding failed: {error:?}"))?;
                let (revisions, changes) = semantic_writes(encoded.objects)?;
                let promoted = self
                    .semantic_store
                    .apply_revisions_and_promote(root, expected, base, &revisions, &changes)
                    .map_err(|error| format!("recovered plan migration failed: {error:?}"))?;
                expected = Some(promoted);
                rebuild_indexes = true;
            }
        }
        if expected.is_some() && rebuild_indexes {
            node_index::rebuild_plan_node_index(self, session, plan.id)?;
        }
        if recovery_pending {
            self.set_legacy_migration_pending(session, plan.id, false)?;
        }
        Ok(())
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
/// # Returns
/// The stable universal root name.
///
/// # Errors
/// Returns an error if the generated name is invalid.
fn plan_root(session: EntityId, plan: EntityId) -> Result<RootName, String> {
    RootName::new(format!("scriberig-plan-{session}-{plan}"))
        .map_err(|error| format!("invalid plan root name: {error:?}"))
}

/// Converts a storage diagnostic to the plan repository error type.
///
/// # Parameters
/// - `message`: Bounded persistence diagnostic.
/// # Returns
/// The plan persistence error.
///
/// A typed plan persistence error.
fn plan_persistence(message: impl Into<String>) -> PlanStoreError {
    PlanStoreError::Persistence(message.into())
}
