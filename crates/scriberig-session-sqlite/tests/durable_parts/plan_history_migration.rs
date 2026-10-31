//! Verifies multi-revision legacy plans migrate into semantic snapshots.

use super::*;

/// Builds a legacy aggregate containing two historical plan revisions.
///
/// # Parameters
/// - `session_id`: Stable owning session identity.
/// - `branch_id`: Legacy root branch identity.
/// - `plan_id`: Stable plan identity.
/// - `node_id`: Stable node identity retained across all revisions.
///
/// # Returns
/// A legacy session whose plan history contains distinct graph snapshots.
fn legacy_session(
    session_id: EntityId,
    branch_id: EntityId,
    plan_id: EntityId,
    node_id: EntityId,
) -> Session {
    let mut plan = Plan::new(
        plan_id,
        session_id,
        scriberig_session::ContinuationAnchor::BranchHead { branch: branch_id },
        "legacy history migration",
    )
    .expect("plan");
    plan.add_node(PlanNode {
        id: node_id,
        parent: Some(scriberig_session::PlanNodeParent::Branch(branch_id)),
        title: "current title".to_owned(),
        purpose: "preserve historical content".to_owned(),
        expected_result: None,
        semantic_scope: Vec::new(),
        status: PlanNodeStatus::Completed,
        branch: Some(EntityId::from_bytes([186; 16])),
        result: Some(EntityId::from_bytes([187; 16])),
        superseded_by: Some(EntityId::from_bytes([188; 16])),
        abandon_reason: None,
        last_failure: None,
        assignment: Some(scriberig_session::ModelAssignment {
            provider_id: "legacy-provider".to_owned(),
            model_id: "legacy-model".to_owned(),
            thinking_level: Some("high".to_owned()),
        }),
        claim: None,
        claim_generation: 2,
    })
    .expect("node");
    plan.add_node(PlanNode {
        id: EntityId::from_bytes([188; 16]),
        parent: Some(scriberig_session::PlanNodeParent::Node(node_id)),
        title: "dependent current node".to_owned(),
        purpose: "preserve dependency topology".to_owned(),
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
    .expect("dependent node");
    plan.add_dependency(node_id, EntityId::from_bytes([188; 16]))
        .expect("current dependency");
    plan.dependencies[0].fulfilled_by = Some(EntityId::from_bytes([187; 16]));
    plan.coordinator = Some(branch_id);
    plan.coordinator_cursor = Some(EntityId::from_bytes([188; 16]));
    plan.coordinator_generation = 5;
    let mut first = plan.nodes[0].clone();
    "first historical title".clone_into(&mut first.title);
    first.status = PlanNodeStatus::Planned;
    first.claim_generation = 0;
    let mut second = plan.nodes[0].clone();
    "second historical title".clone_into(&mut second.title);
    second.status = PlanNodeStatus::Completed;
    second.branch = Some(EntityId::from_bytes([184; 16]));
    second.result = Some(EntityId::from_bytes([185; 16]));
    second.claim_generation = 1;
    plan.history = vec![
        scriberig_session::PlanRevision {
            revision: 1,
            goal: "first goal".to_owned(),
            status: scriberig_session::PlanStatus::Planned,
            nodes: vec![first],
            dependencies: Vec::new(),
            summaries: Vec::new(),
            evidence: Vec::new(),
            gates: Vec::new(),
            reason: None,
        },
        scriberig_session::PlanRevision {
            revision: 2,
            goal: "second goal".to_owned(),
            status: scriberig_session::PlanStatus::Running,
            nodes: vec![second],
            dependencies: Vec::new(),
            summaries: Vec::new(),
            evidence: Vec::new(),
            gates: Vec::new(),
            reason: Some("recovered after failed attempt".to_owned()),
        },
    ];
    plan.revision = 3;
    Session {
        id: session_id,
        branches: vec![Branch::new(branch_id, "root").expect("branch")],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: vec![plan],
        execution: scriberig_session::SessionExecutionSelection::default(),
    }
}

/// Preserves historical node state across aggregate-to-semantic migration.
#[test]
fn legacy_plan_history_preserves_multiple_revisions_after_restart() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-plan-history-migration-{}.sqlite",
        EntityId::new()
    ));
    let session_id = EntityId::from_bytes([180; 16]);
    let branch_id = EntityId::from_bytes([181; 16]);
    let plan_id = EntityId::from_bytes([182; 16]);
    let node_id = EntityId::from_bytes([183; 16]);
    let session = legacy_session(session_id, branch_id, plan_id, node_id);
    let legacy = session.plans[0].clone();
    let store = SqliteSessionStore::open(&path).expect("open store");
    store.save(&session).expect("migrate legacy plan");
    drop(store);

    let reopened = SqliteSessionStore::open(&path).expect("reopen store");
    let loaded = reopened
        .load(session_id)
        .expect("load migrated session")
        .expect("session exists");
    let migrated = &loaded.plans[0];
    assert_eq!(migrated.id, legacy.id);
    assert_eq!(migrated.goal, legacy.goal);
    assert_eq!(migrated.root_branch, legacy.root_branch);
    assert_eq!(migrated.nodes, legacy.nodes);
    assert_eq!(migrated.dependencies, legacy.dependencies);
    assert_eq!(migrated.coordinator, legacy.coordinator);
    assert_eq!(migrated.coordinator_cursor, legacy.coordinator_cursor);
    assert_eq!(
        migrated.coordinator_generation,
        legacy.coordinator_generation
    );
    assert_eq!(migrated.revision, legacy.revision);
    assert_eq!(migrated.revision, 3);
    assert_eq!(migrated.history.len(), 3);
    assert_eq!(migrated.history[0].goal, "first goal");
    assert_eq!(migrated.history[0].nodes[0].title, "first historical title");
    assert_eq!(migrated.history[1].goal, "second goal");
    assert_eq!(
        migrated.history[1].nodes[0].status,
        PlanNodeStatus::Completed
    );
    assert_eq!(migrated.history[1].nodes[0].claim_generation, 1);
    assert_eq!(migrated.history[1].nodes[0].id, node_id);
    assert_eq!(
        migrated.history[1].nodes[0].branch,
        Some(EntityId::from_bytes([184; 16]))
    );
    assert_eq!(
        migrated.history[1].nodes[0].result,
        Some(EntityId::from_bytes([185; 16]))
    );
    assert_eq!(
        migrated.history[1].reason.as_deref(),
        Some("recovered after failed attempt")
    );
    assert_eq!(migrated.history[2].nodes[0].title, "current title");
    assert_eq!(migrated.history[2].nodes[0].claim_generation, 2);
    assert_eq!(
        migrated.history[2].nodes[0].branch,
        Some(EntityId::from_bytes([186; 16]))
    );
    assert_eq!(
        migrated.history[2].nodes[0].result,
        Some(EntityId::from_bytes([187; 16]))
    );
    drop(reopened);
    let _ = std::fs::remove_file(path);
}
