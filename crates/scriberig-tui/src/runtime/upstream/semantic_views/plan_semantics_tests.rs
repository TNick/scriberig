//! Verifies the Plans view renders shared `PlanView` status and rollup semantics.
//!
//! Every case compares the projected rows and details with the views assembled by
//! `scriberig-session`, the same assembly the shared application plan read and
//! the server plan view consume.

use std::str::FromStr;

use scriberig_session::{
    PlanEvidenceOwner, PlanEvidenceRole, PlanNodeParent, PlanNodeStatus, PlanRevision, PlanStatus,
};

use super::test_support::{
    abandoned_node, evidence, node_with_status, plan_rows, required_child_gate, sample_plan,
    selected_details, session_with_plans, shared_node_view, status_total, validation_entry_gate,
};
use super::*;

#[path = "plan_semantics_tests/attempt_history.rs"]
mod attempt_history;

/// Verifies a deep required-child failure reaches the parent as the shared view's
/// effective status, which a scan of the parent's own projected children cannot
/// produce because that child row is itself `Completed`.
#[test]
fn deep_required_child_failure_shares_the_effective_status_not_a_row_scan() {
    let plan_id = EntityId::from_bytes([11; 16]);
    let branch = EntityId::from_bytes([12; 16]);
    let root = EntityId::from_bytes([13; 16]);
    let child = EntityId::from_bytes([14; 16]);
    let leaf = EntityId::from_bytes([15; 16]);

    let mut plan = sample_plan(plan_id, branch, root, child);
    plan.nodes = vec![
        node_with_status(root, None, PlanNodeStatus::Completed),
        node_with_status(
            child,
            Some(PlanNodeParent::Node(root)),
            PlanNodeStatus::Completed,
        ),
        node_with_status(
            leaf,
            Some(PlanNodeParent::Node(child)),
            PlanNodeStatus::Failed,
        ),
    ];
    plan.root_nodes = vec![root];
    plan.dependencies = Vec::new();
    plan.gates = vec![
        required_child_gate(EntityId::from_bytes([16; 16]), root, child),
        required_child_gate(EntityId::from_bytes([17; 16]), child, leaf),
    ];
    let (rows, session) = plan_rows(plan.clone());

    let view = shared_node_view(&session, &plan, root);
    assert_eq!(view.direct_status, PlanNodeStatus::Completed);
    assert_eq!(
        view.effective_status,
        PlanNodeStatus::Failed,
        "the shared rollup must carry the deep failure to the root"
    );

    let root_row = rows
        .iter()
        .find(|row| row.selection.node == Some(root))
        .expect("root node row");
    let child_row = rows
        .iter()
        .find(|row| row.selection.node == Some(child))
        .expect("child node row");
    assert!(
        child_row.label.starts_with("Completed"),
        "the projected child row shows its direct status only: {:?}",
        child_row.label
    );
    assert_eq!(
        root_row.label,
        node_label(&root_row.semantics, &plan.nodes[0]),
        "the row label is the shared node view's label"
    );
    assert!(
        root_row
            .label
            .contains(&format!("effective {:?}", view.effective_status)),
        "the root row must show the shared effective status: {:?}",
        root_row.label
    );

    let details = selected_details(&session, &rows, plan_id, Some(root));
    assert!(details.contains("Status: Completed"), "{details}");
    assert!(
        details.contains(&format!("Effective status: {:?}", view.effective_status)),
        "{details}"
    );
}

/// Verifies the upstream TUI projection agrees with the shared plan-view fixture.
#[test]
fn tui_node_projection_matches_shared_plan_view_fixture() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../tests/conformance/fixtures/plan-view.json"
    ))
    .expect("shared plan-view fixture");
    let plan_fixture = &fixture["plan"];
    let node_fixture = &fixture["nodes"][0];
    let id = |value: &serde_json::Value| {
        EntityId::from_str(value.as_str().expect("entity id")).expect("valid entity id")
    };
    let plan_id = id(&plan_fixture["id"]);
    let node_id = id(&node_fixture["id"]);
    let branch = EntityId::from_str(
        node_fixture["parent"]
            .as_str()
            .expect("node parent")
            .strip_prefix("branch:")
            .expect("branch parent prefix"),
    )
    .expect("valid branch identity");
    let snapshot = scriberig_core::SnapshotId::from_str(
        plan_fixture["snapshot"].as_str().expect("plan snapshot"),
    )
    .expect("valid snapshot identity");
    let mut plan = sample_plan(plan_id, branch, node_id, EntityId::from_bytes([99; 16]));
    plan.status = PlanStatus::Planned;
    plan.source_snapshot = Some(snapshot);
    plan.nodes = vec![node_with_status(
        node_id,
        Some(PlanNodeParent::Branch(branch)),
        PlanNodeStatus::Ready,
    )];
    plan.root_nodes = vec![node_id];
    plan.dependencies.clear();
    let gate_id = EntityId::from_bytes([98; 16]);
    plan.gates = vec![validation_entry_gate(gate_id, node_id)];
    let mut validation = evidence(
        EntityId::from_bytes([97; 16]),
        PlanEvidenceOwner::Gate(gate_id),
        PlanEvidenceRole::Supports,
        EntityId::from_bytes([96; 16]),
    );
    validation
        .evidence
        .attributes
        .insert("passed".to_owned(), "true".to_owned());
    plan.evidence = vec![validation];
    let session = session_with_plans(vec![plan.clone()]);
    let actual = shared_node_view(&session, &plan, node_id);

    assert_eq!(actual.id, node_id);
    assert_eq!(actual.plan, plan_id);
    assert_eq!(actual.snapshot, Some(snapshot));
    assert_eq!(actual.parent, Some(PlanNodeParent::Branch(branch)));
    assert_eq!(actual.direct_status, PlanNodeStatus::Ready);
    assert_eq!(actual.effective_status, PlanNodeStatus::Ready);
    assert_eq!(actual.entry_gates.satisfied, 1);
    assert_eq!(actual.evidence_count, 1);
    assert!(!actual.can_retry);
    assert_eq!(actual.dependency_blockers.len(), 0);
    assert_eq!(actual.id.to_string(), node_fixture["id"]);
    assert_eq!(
        format!("{:?}", actual.direct_status),
        node_fixture["direct_status"]
    );
    assert_eq!(
        format!("{:?}", actual.effective_status),
        node_fixture["effective_status"]
    );
    assert_eq!(
        actual.entry_gates.satisfied,
        node_fixture["entry_gates"]["satisfied"]
    );
    assert_eq!(actual.evidence_count, node_fixture["evidence_count"]);
    assert_eq!(actual.can_retry, node_fixture["can_retry"]);
    assert_eq!(
        actual.dependency_blockers.len(),
        node_fixture["dependency_blockers"]
            .as_array()
            .expect("dependency blocker list")
            .len()
    );
}

/// Verifies ignored and abandoned descendants mark ancestors without changing
/// the ancestor's own canonical status.
#[test]
fn ignored_descendant_markers_come_from_the_shared_rollup() {
    let plan_id = EntityId::from_bytes([51; 16]);
    let branch = EntityId::from_bytes([52; 16]);
    let root = EntityId::from_bytes([53; 16]);
    let child = EntityId::from_bytes([54; 16]);
    let leaf = EntityId::from_bytes([55; 16]);

    let mut plan = sample_plan(plan_id, branch, root, child);
    plan.nodes = vec![
        node_with_status(root, None, PlanNodeStatus::Planned),
        node_with_status(
            child,
            Some(PlanNodeParent::Node(root)),
            PlanNodeStatus::Planned,
        ),
        abandoned_node(
            leaf,
            Some(PlanNodeParent::Node(child)),
            scriberig_session::PlanAbandonReason::Ignored,
        ),
    ];
    plan.root_nodes = vec![root];
    plan.dependencies = Vec::new();
    let (rows, session) = plan_rows(plan.clone());

    let view = shared_node_view(&session, &plan, root);
    assert!(view.contains_ignored);
    assert!(view.contains_abandoned);
    assert_eq!(view.direct_status, PlanNodeStatus::Planned);
    assert_eq!(view.effective_status, PlanNodeStatus::Planned);

    let root_row = rows
        .iter()
        .find(|row| row.selection.node == Some(root))
        .expect("root node row");
    assert!(
        root_row.label.contains("contains ignored")
            && root_row.label.contains("contains abandoned"),
        "the shared subtree markers reach the ancestor row: {:?}",
        root_row.label
    );
    assert!(
        root_row.label.starts_with("Planned"),
        "the ancestor keeps its own canonical direct status: {:?}",
        root_row.label
    );
    assert!(
        !root_row.label.contains("effective"),
        "no derived status is invented when the shared rollup matches: {:?}",
        root_row.label
    );
}

/// Verifies plan details expose the shared exact revision, the separate plan
/// mutation revision, status counts, and root identities.
#[test]
fn plan_details_expose_shared_revision_status_and_root_summaries() {
    let plan_id = EntityId::from_bytes([71; 16]);
    let branch = EntityId::from_bytes([72; 16]);
    let first = EntityId::from_bytes([73; 16]);
    let second = EntityId::from_bytes([74; 16]);

    let mut plan = sample_plan(plan_id, branch, first, second);
    plan.nodes = vec![
        node_with_status(first, None, PlanNodeStatus::Running),
        node_with_status(
            second,
            Some(PlanNodeParent::Node(first)),
            PlanNodeStatus::Planned,
        ),
    ];
    let (rows, session) = plan_rows(plan.clone());
    let view = plan
        .plan_view(plan.source_snapshot)
        .expect("shared plan view");

    assert_eq!(view.node_count, status_total(view.status_counts));
    assert_eq!(view.root_nodes, vec![first]);

    let header = rows
        .iter()
        .find(|row| row.selection.node.is_none())
        .expect("plan header row");
    assert!(
        header
            .label
            .contains(&format!("r{}", view.mutation_revision)),
        "the header names the plan mutation revision: {:?}",
        header.label
    );

    let details = selected_details(&session, &rows, plan_id, None);
    for expected in [
        &format!("Exact revision: {}", view.revision),
        &format!("Mutation revision: {}", view.mutation_revision),
        "Nodes: 2 · planned 1 · ready 0 · running 1 · completed 0 · failed 0 · abandoned 0",
        &format!("Root nodes: {first}"),
        &format!("Coordinator node: {first}"),
    ] {
        assert!(
            details.contains(expected),
            "missing {expected:?} in {details}"
        );
    }
    assert_ne!(
        view.revision.to_string(),
        view.mutation_revision.to_string(),
        "the exact immutable revision stays distinct from the plan mutation revision"
    );
}

/// Verifies a plan copy the shared assembly rejects keeps stable identity rows
/// and an explicit notice instead of terminal-local plan semantics.
#[test]
fn a_rejected_shared_assembly_reports_unavailable_semantics() {
    let plan_id = EntityId::from_bytes([81; 16]);
    let branch = EntityId::from_bytes([82; 16]);
    let present = EntityId::from_bytes([83; 16]);
    let missing = EntityId::from_bytes([84; 16]);

    let mut plan = sample_plan(plan_id, branch, present, missing);
    plan.nodes = vec![node_with_status(
        present,
        Some(PlanNodeParent::Node(missing)),
        PlanNodeStatus::Ready,
    )];
    plan.root_nodes = vec![present];
    plan.dependencies = Vec::new();
    let (rows, session) = plan_rows(plan.clone());

    assert_eq!(rows.len(), 2, "the identity rows still project");
    let node_row = rows
        .iter()
        .find(|row| row.selection.node == Some(present))
        .expect("node row");
    assert!(
        node_row
            .label
            .contains(&format!("{present} · shared plan semantics unavailable")),
        "no local status is substituted: {:?}",
        node_row.label
    );
    tree_from_items(
        &rows
            .iter()
            .map(|row| TreeItem {
                id: row.id.clone(),
                label: row.id.clone(),
                parent_id: row.parent_id.clone(),
            })
            .collect::<Vec<_>>(),
    )
    .expect("upstream tree accepts the identity-only projection");

    let details = selected_details(&session, &rows, plan_id, Some(present));
    assert!(
        details.contains("Shared plan semantics: unavailable for this node"),
        "{details}"
    );
    assert!(
        !details.contains("Effective status:"),
        "the projection must not publish derived status it did not receive: {details}"
    );
}

/// Verifies the selected node's shared semantics render above the plan history
/// list, so a multi-revision plan cannot scroll them out of the bounded pane.
#[test]
fn selected_node_semantics_stay_above_the_plan_history_list() {
    let plan_id = EntityId::from_bytes([121; 16]);
    let branch = EntityId::from_bytes([122; 16]);
    let root = EntityId::from_bytes([123; 16]);
    let child = EntityId::from_bytes([124; 16]);

    let mut plan = sample_plan(plan_id, branch, root, child);
    plan.nodes = vec![
        node_with_status(root, None, PlanNodeStatus::Running),
        node_with_status(
            child,
            Some(PlanNodeParent::Node(root)),
            PlanNodeStatus::Ready,
        ),
    ];
    plan.root_nodes = vec![root];
    plan.dependencies = Vec::new();
    for revision in 0..4 {
        plan.history.push(PlanRevision {
            revision,
            goal: "Earlier graph goal".to_owned(),
            status: PlanStatus::Draft,
            nodes: Vec::new(),
            dependencies: Vec::new(),
            summaries: Vec::new(),
            evidence: Vec::new(),
            gates: Vec::new(),
            reason: None,
        });
    }
    let (rows, session) = plan_rows(plan);

    let details = selected_details(&session, &rows, plan_id, Some(child));
    let history_position = details
        .find("## Prior plan revisions")
        .expect("immutable plan history stays inspectable");
    assert!(
        details
            .find(&format!("Structural parent node: {root}"))
            .expect("the selected node reports its structural parent")
            < history_position,
        "the selected node's semantics must stay above the history list: {details}"
    );
}
