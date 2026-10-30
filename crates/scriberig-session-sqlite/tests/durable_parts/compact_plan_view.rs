//! Verifies bounded plan reads avoid detailed semantic payloads.

use std::collections::BTreeMap;

use rusqlite::params;
use scriberig_core::EntityId;
use scriberig_session::{
    Branch, ContinuationAnchor, Plan, PlanEvidenceAssociation, PlanEvidenceKind, PlanEvidenceOwner,
    PlanEvidenceReference, PlanEvidenceRole, PlanNode, PlanNodeParent, PlanNodeStatus,
    PlanSemanticReference, Session, SessionStore,
};
use scriberig_session_sqlite::SqliteSessionStore;

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "The regression contrasts compact reads with corrupt detailed payloads."
)]
/// Reads plan metadata when an unrelated evidence payload is unreadable.
fn compact_plan_view_skips_detailed_evidence_payloads() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-compact-plan-view-{}.sqlite",
        EntityId::new()
    ));
    let session_id = EntityId::from_bytes([71; 16]);
    let branch_id = EntityId::from_bytes([72; 16]);
    let plan_id = EntityId::from_bytes([73; 16]);
    let node_id = EntityId::from_bytes([74; 16]);
    let evidence_id = EntityId::from_bytes([75; 16]);
    let store = SqliteSessionStore::open(&path).expect("open session store");
    let mut plan = Plan::new(
        plan_id,
        session_id,
        ContinuationAnchor::BranchHead { branch: branch_id },
        "bounded plan view",
    )
    .expect("plan");
    plan.add_node(PlanNode {
        id: node_id,
        parent: Some(PlanNodeParent::Branch(branch_id)),
        title: "inspect summary".to_owned(),
        purpose: "load only compact state".to_owned(),
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
    plan.attach_evidence(PlanEvidenceAssociation {
        id: evidence_id,
        owner: PlanEvidenceOwner::Node(node_id),
        role: PlanEvidenceRole::Informational,
        evidence: PlanEvidenceReference {
            kind: PlanEvidenceKind::Artifact,
            reference: PlanSemanticReference {
                entity: EntityId::from_bytes([76; 16]),
                revision: None,
                snapshot: None,
            },
            attributes: BTreeMap::default(),
        },
        note: None,
        actor: None,
    })
    .expect("evidence");
    let encoded =
        scriberig_session::PlanSemanticSnapshot::from_plan(&plan).expect("semantic plan encoding");
    let evidence_revision = encoded
        .objects
        .iter()
        .find(|object| object.entity == evidence_id)
        .expect("evidence semantic object")
        .revision;
    store
        .save(&Session {
            id: session_id,
            branches: vec![Branch::new(branch_id, "root").expect("branch")],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![plan],
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("persist session");
    let snapshot = store
        .load(session_id)
        .expect("load session")
        .expect("stored session")
        .plans[0]
        .source_snapshot
        .expect("semantic snapshot");

    let connection = rusqlite::Connection::open(&path).expect("open corruption connection");
    connection
        .execute(
            "UPDATE revisions SET payload=?2 WHERE id=?1",
            params![evidence_revision.as_bytes().as_slice(), vec![0_u8]],
        )
        .expect("corrupt detail payload");
    drop(connection);

    let compact = store
        .load_plan_view_at_snapshot(session_id, plan_id, snapshot)
        .expect("read compact plan view")
        .expect("snapshot belongs to plan");
    assert_eq!(compact.id, plan_id);
    assert_eq!(compact.snapshot, Some(snapshot));
    assert_eq!(compact.node_count, 1);
    assert_eq!(
        compact.status_counts.planned + compact.status_counts.ready,
        1
    );
    assert!(
        store
            .load_plan_at_snapshot(session_id, plan_id, snapshot)
            .is_err()
    );
    drop(store);
    let _ = std::fs::remove_file(path);
}
