//! Verifies application access to exact-snapshot attempt history pages.

use super::*;
use scriberig_session::{
    Branch, Plan, PlanNode, PlanNodeParent, PlanNodeStatus, Session, SessionStore,
};
use scriberig_session_sqlite::SqliteSessionStore;
use std::sync::Arc;

/// Reads a bounded attempt page through application authorization.
#[test]
fn application_reads_plan_attempt_page_and_rejects_unbounded_limit() {
    let session_id = EntityId::from_bytes([171; 16]);
    let root_id = EntityId::from_bytes([172; 16]);
    let plan_id = EntityId::from_bytes([173; 16]);
    let node_id = EntityId::from_bytes([174; 16]);
    let mut plan = Plan::new(
        plan_id,
        session_id,
        scriberig_session::ContinuationAnchor::BranchHead { branch: root_id },
        "read attempts",
    )
    .expect("plan");
    plan.add_node(PlanNode {
        id: node_id,
        parent: Some(PlanNodeParent::Branch(root_id)),
        title: "attempt history".to_owned(),
        purpose: "test a bounded history read".to_owned(),
        expected_result: None,
        semantic_scope: Vec::new(),
        status: PlanNodeStatus::Planned,
        branch: None,
        result: None,
        superseded_by: None,
        abandon_reason: None,
        last_failure: None,
        assignment: None,
        claim: None,
        claim_generation: 0,
    })
    .expect("node");
    plan.transition(scriberig_session::PlanStatus::Planned)
        .expect("plan ready");
    let store = Arc::new(SqliteSessionStore::open_in_memory().expect("store"));
    store
        .save(&Session {
            id: session_id,
            branches: vec![Branch::new(root_id, "root").expect("root")],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![plan],
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("persist plan");
    let snapshot = store
        .load(session_id)
        .expect("load session")
        .expect("session")
        .plans[0]
        .source_snapshot
        .expect("semantic snapshot");
    let service = ApplicationService::new(Fixture).with_session_store(store);
    let request = scriberig_app::PlanAttemptHistoryRequest {
        session: session_id,
        plan: plan_id,
        node: node_id,
        snapshot,
        offset: 0,
        limit: 10,
    };

    assert_eq!(
        service
            .read_plan_attempt_history(&session_context(), &request)
            .expect("read attempt page"),
        Vec::<scriberig_session::PlanSemanticAttempt>::new()
    );
    assert!(matches!(
        service.read_plan_attempt_history(
            &session_context(),
            &scriberig_app::PlanAttemptHistoryRequest {
                limit: 0,
                ..request
            }
        ),
        Err(ApplicationError::InvalidRequest { .. })
    ));
}
