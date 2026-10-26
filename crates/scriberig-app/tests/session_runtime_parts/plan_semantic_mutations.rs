//! Verifies shared application plan mutation behavior.

use super::*;

#[path = "plan_semantic_mutations/revision_boundary.rs"]
mod revision_boundary;

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "This regression compares the SQLite compact projection with the shared semantic fixture while proving exact-snapshot detail remains available."
)]
/// Verifies compact views preserve gate and evidence summaries from SQLite.
fn compact_plan_view_reads_cached_gate_summaries() {
    let conformance: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/conformance/fixtures/plan-view.json"
    ))
    .expect("shared plan view fixture");
    let expected_node = &conformance["nodes"][0];
    let path = std::env::temp_dir().join(format!(
        "scriberig-compact-plan-view-{}.sqlite",
        EntityId::new()
    ));
    let store = std::sync::Arc::new(
        scriberig_session_sqlite::SqliteSessionStore::open(&path).expect("open compact plan store"),
    );
    let session_store: std::sync::Arc<dyn scriberig_session::SessionStore> = store.clone();
    let service = ApplicationService::new(Fixture).with_session_store(session_store);
    let context = session_context();
    let session = EntityId::from_bytes([0x72; 16]);
    service
        .start_agent(&context, session, "seed plan details")
        .expect("seed session");
    let state = service
        .read_session(&context, &SessionRequest { session })
        .expect("read session");
    let plan = service
        .create_plan(
            &context,
            &PlanCreateRequest {
                session,
                anchor: scriberig_session::ContinuationAnchor::BranchHead {
                    branch: state.branches[0].id,
                },
                goal: "bounded requirement details".to_owned(),
                nodes: vec![PlanNodeDraft {
                    key: "work".to_owned(),
                    parent_index: None,
                    title: "Work".to_owned(),
                    purpose: "inspect requirements".to_owned(),
                    expected_result: None,
                    semantic_scope: Vec::new(),
                    assignment: None,
                }],
                dependencies: Vec::new(),
            },
        )
        .expect("create plan");
    let owner = plan.nodes[0].id;
    let plan = service
        .mutate_plan(
            &context,
            &PlanMutationRequest {
                session,
                plan: plan.id,
                expected_revision: plan.revision,
                mutation: PlanMutation::AddGate {
                    owner,
                    phase: scriberig_session::PlanGatePhase::Entry,
                    requirement: scriberig_session::PlanGateRequirement::EvidencePresent {
                        kind: scriberig_session::PlanEvidenceKind::ValidationResult,
                    },
                },
            },
        )
        .expect("add entry gate");
    let gate = plan.gates[0].id;
    let plan = service
        .mutate_plan(
            &context,
            &PlanMutationRequest {
                session,
                plan: plan.id,
                expected_revision: plan.revision,
                mutation: PlanMutation::AttachEvidence {
                    owner: scriberig_session::PlanEvidenceOwner::Gate(gate),
                    role: scriberig_session::PlanEvidenceRole::Supports,
                    evidence: scriberig_session::PlanEvidenceReference {
                        kind: scriberig_session::PlanEvidenceKind::ValidationResult,
                        reference: scriberig_session::PlanSemanticReference {
                            entity: state.branches[0].id,
                            revision: Some(scriberig_core::RevisionId::from_bytes([73; 32])),
                            snapshot: Some(scriberig_core::SnapshotId::from_bytes([74; 32])),
                        },
                        attributes: std::collections::BTreeMap::new(),
                    },
                    note: None,
                },
            },
        )
        .expect("attach supporting evidence");

    let (view, nodes) = service
        .read_plan_views(&context, session, plan.id, None)
        .expect("read compact plan views");

    assert_eq!(view.node_count, 1);
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].id, owner);
    assert_eq!(view.id.to_string(), conformance["plan"]["id"]);
    assert_eq!(nodes[0].id.to_string(), expected_node["id"]);
    assert_eq!(
        nodes[0].direct_status,
        scriberig_session::PlanNodeStatus::Ready
    );
    assert_eq!(
        nodes[0].effective_status,
        scriberig_session::PlanNodeStatus::Ready
    );
    assert_eq!(nodes[0].entry_gates.satisfied, 1);
    assert_eq!(nodes[0].entry_gates.pending, 0);
    assert_eq!(nodes[0].evidence_count, 1);
    assert_eq!(
        serde_json::to_value(nodes[0].entry_gates.satisfied).expect("serialize gate count"),
        expected_node["entry_gates"]["satisfied"]
    );
    assert_eq!(
        serde_json::to_value(nodes[0].evidence_count).expect("serialize evidence count"),
        expected_node["evidence_count"]
    );
    assert_eq!(
        serde_json::to_value(nodes[0].can_retry).expect("serialize retry outcome"),
        expected_node["can_retry"]
    );
    assert_eq!(
        serde_json::to_value(&nodes[0].dependency_blockers).expect("serialize dependency blockers"),
        expected_node["dependency_blockers"]
    );
    let node = &nodes[0];
    let parent = node.parent.map(|parent| match parent {
        scriberig_session::PlanNodeParent::Branch(id) => format!("branch:{id}"),
        scriberig_session::PlanNodeParent::Node(id) => format!("node:{id}"),
    });
    let application_projection = serde_json::json!({
        "plan": {
            "id": view.id.to_string(),
            "revision": view.revision.to_string(),
            "snapshot": view.snapshot.map(|snapshot| snapshot.to_string()),
            "mutation_revision": view.mutation_revision,
            "goal": view.goal,
            "status": format!("{:?}", view.status),
            "coordinator": view.coordinator.map(|id| id.to_string()),
            "coordinator_cursor": view.coordinator_cursor.map(|id| id.to_string()),
            "root_nodes": view.root_nodes.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "node_count": view.node_count,
            "status_counts": serde_json::to_value(view.status_counts).expect("status counts"),
        },
        "nodes": [{
            "id": node.id.to_string(),
            "revision": node.revision.to_string(),
            "snapshot": node.snapshot.map(|snapshot| snapshot.to_string()),
            "plan": node.plan.to_string(),
            "parent": parent,
            "title": node.title,
            "purpose": node.purpose,
            "direct_status": format!("{:?}", node.direct_status),
            "effective_status": format!("{:?}", node.effective_status),
            "contains_ignored": node.contains_ignored,
            "contains_abandoned": node.contains_abandoned,
            "summary_id": node.summary_id.map(|id| id.to_string()),
            "summary_preview": node.summary_preview,
            "branch": node.branch.map(|id| id.to_string()),
            "result": node.result.map(|id| id.to_string()),
            "assignment": serde_json::to_value(&node.assignment).expect("assignment"),
            "dependency_blockers": node.dependency_blockers.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "entry_gates": serde_json::to_value(node.entry_gates).expect("entry gate counts"),
            "completion_gates": serde_json::to_value(node.completion_gates).expect("completion gate counts"),
            "evidence_count": node.evidence_count,
            "can_retry": node.can_retry,
            "last_failure": serde_json::to_value(&node.last_failure).expect("last failure"),
        }]
    });
    assert_eq!(application_projection, conformance);
    drop(service);
    drop(store);
    let _ = std::fs::remove_file(path);
}
