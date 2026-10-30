//! Verifies paged attempt, gate, and evidence reads after retry.

use scriberig_core::{EntityId, RootName};
use scriberig_session::{
    PlanNodeStatus, PlanSemanticRevision, PlanSemanticSnapshot, Session, SessionStore,
};
use scriberig_store::{RevisionStore, RootStore, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;

use super::super::SqliteSessionStore;

/// Verifies retry provenance and exact dependency fulfillment after reload.
///
/// # Parameters
/// - `session`: Reloaded durable session containing both attempts.
/// - `node`: Stable node identity shared by both attempts.
/// - `first_branch`: Failed attempt branch.
/// - `second_branch`: Successful retry branch.
/// - `failure`: Exact failure result identity.
/// - `success`: Exact successful result identity.
pub(super) fn assert_retry_provenance(
    session: &Session,
    node: EntityId,
    first_branch: EntityId,
    second_branch: EntityId,
    failure: EntityId,
    success: EntityId,
) {
    let plan = &session.plans[0];
    let attempts = plan.attempt_history(node).expect("known node history");
    assert_eq!(attempts.len(), 2);
    assert_eq!(attempts[0].branch, first_branch);
    assert_eq!(attempts[0].claim_generation, 1);
    assert_eq!(attempts[0].status, PlanNodeStatus::Failed);
    assert_eq!(
        attempts[0]
            .failure
            .as_ref()
            .map(|reference| reference.reference.entity),
        Some(failure)
    );
    assert_eq!(attempts[1].branch, second_branch);
    assert_eq!(attempts[1].claim_generation, 2);
    assert_eq!(attempts[1].status, PlanNodeStatus::Completed);
    assert_eq!(attempts[1].result, Some(success));
    assert_eq!(attempts[1].failure, None);
    assert_eq!(plan.dependencies.len(), 1);
    assert_eq!(
        plan.dependencies[0].fulfilled_by,
        Some(success),
        "the successful retry must fulfill the dependency with its exact result"
    );
    assert_eq!(
        session.branches[1].result.as_ref().map(|result| result.id),
        Some(failure)
    );
    assert_eq!(
        session.branches[2].result.as_ref().map(|result| result.id),
        Some(success)
    );
}

/// Holds the stores and identities needed to inspect attempt pages.
///
/// # Public members
/// - `store`: Durable session repository.
/// - `semantic_store`: Universal immutable-object store.
/// - `session`: Owning session identity.
/// - `plan`: Stable plan identity.
/// - `node`: Stable plan-node identity.
/// - `gate`: Stable completion-gate identity.
/// - `evidence`: Stable evidence-association identity.
/// - `first_branch`: Failed attempt branch identity.
/// - `second_branch`: Successful retry branch identity.
///
/// # Private members
/// None.
pub(super) struct AttemptPageFixture<'a> {
    /// Durable session repository.
    pub(super) store: &'a SqliteSessionStore,
    /// Universal immutable-object store.
    pub(super) semantic_store: &'a SqliteStore,
    /// Owning session identity.
    pub(super) session: EntityId,
    /// Stable plan identity.
    pub(super) plan: EntityId,
    /// Stable plan-node identity.
    pub(super) node: EntityId,
    /// Stable completion-gate identity.
    pub(super) gate: EntityId,
    /// Stable evidence-association identity.
    pub(super) evidence: EntityId,
    /// Failed attempt branch identity.
    pub(super) first_branch: EntityId,
    /// Successful retry branch identity.
    pub(super) second_branch: EntityId,
}

impl AttemptPageFixture<'_> {
    /// Checks exact-snapshot pages for both attempts and attached details.
    ///
    /// # Parameters
    /// - `self`: Stores and identities used by this assertion.
    pub(super) fn assert_attempt_pages(&self) {
        let root = RootName::new(format!("scriberig-plan-{}-{}", self.session, self.plan))
            .expect("plan root");
        let snapshot = self
            .semantic_store
            .read_root(&root)
            .expect("current plan root");
        let objects = self
            .semantic_store
            .iter(snapshot)
            .expect("read current semantic plan")
            .into_iter()
            .map(|(entity, revision_id)| {
                let revision = self
                    .semantic_store
                    .get(revision_id)
                    .expect("read semantic revision")
                    .expect("semantic revision exists");
                PlanSemanticRevision {
                    entity,
                    object_type: revision.object_type().clone(),
                    revision: revision.id(),
                    payload: revision.payload().to_vec(),
                }
            })
            .collect();
        let semantic_snapshot = PlanSemanticSnapshot {
            plan: self.plan,
            snapshot: Some(snapshot),
            objects,
        };
        let semantic_attempts = semantic_snapshot.attempts().expect("semantic attempts");
        let first_page = self
            .store
            .load_plan_attempts_at_snapshot(self.session, self.plan, self.node, snapshot, 0, 1)
            .expect("read first attempt page")
            .expect("snapshot belongs to plan");
        let second_page = self
            .store
            .load_plan_attempts_at_snapshot(self.session, self.plan, self.node, snapshot, 1, 1)
            .expect("read second attempt page")
            .expect("snapshot belongs to plan");
        let node_details = self
            .store
            .load_plan_node_details_at_snapshot(self.session, self.plan, self.node, snapshot, 0, 10)
            .expect("read node detail page")
            .expect("snapshot belongs to plan");

        assert_eq!(semantic_attempts.len(), 2);
        assert_eq!(semantic_attempts[0].status, PlanNodeStatus::Failed);
        assert_eq!(semantic_attempts[1].status, PlanNodeStatus::Completed);
        assert_eq!(first_page.len(), 1);
        assert_eq!(first_page[0].branch, self.first_branch);
        assert_eq!(second_page.len(), 1);
        assert_eq!(second_page[0].branch, self.second_branch);
        assert_eq!(node_details.gates.len(), 2);
        let completion_gate = node_details
            .gates
            .iter()
            .find(|candidate| candidate.id == self.gate)
            .expect("completion gate details");
        assert_eq!(
            completion_gate.waiver.as_ref().unwrap().reason,
            "test fixture waiver"
        );
        assert_eq!(node_details.evidence.len(), 2);
        assert!(
            node_details
                .evidence
                .iter()
                .any(|item| item.id == self.evidence)
        );
    }
}
