//! Shared durable plan builders for the plan projection tests.

use std::collections::BTreeMap;

use scriberig_core::EntityId;
use scriberig_session::{
    Branch, BranchResult, BranchStatus, Plan, PlanAbandonReason, PlanEvidenceAssociation,
    PlanEvidenceKind, PlanEvidenceOwner, PlanEvidenceReference, PlanEvidenceRole, PlanGate,
    PlanGatePhase, PlanGateRequirement, PlanNode, PlanNodeParent, PlanNodeStatus,
    PlanNodeStatusCounts, PlanNodeSummary, PlanNodeView, PlanSemanticReference, PlanStatus,
    Session,
};

use crate::selection::PlanSelection;

/// Builds a deterministic plan with structural and dependency relations diverging.
///
/// - `plan_id`: Durable plan identity.
/// - `branch`: Structural parent branch identity.
/// - `first`: First durable node identity.
/// - `second`: Second durable node identity.
///
/// # Returns
/// A plan with one structural edge and a reverse dependency edge.
pub(super) fn sample_plan(
    plan_id: EntityId,
    branch: EntityId,
    first: EntityId,
    second: EntityId,
) -> Plan {
    Plan {
        id: plan_id,
        session: Some(EntityId::from_bytes([9; 16])),
        anchor: None,
        goal: "Verify graph projection".to_owned(),
        revision: 4,
        status: PlanStatus::Running,
        root_branch: branch,
        nodes: vec![
            node(first, None),
            node(second, Some(PlanNodeParent::Node(first))),
        ],
        dependencies: vec![scriberig_session::PlanDependency {
            prerequisite: second,
            dependent: first,
            fulfilled_by: None,
        }],
        source_snapshot: None,
        summaries: Vec::new(),
        evidence: Vec::new(),
        gates: Vec::new(),
        coordinator: Some(branch),
        coordinator_cursor: Some(first),
        coordinator_claim: None,
        coordinator_generation: 1,
        root_nodes: vec![first],
        history: Vec::new(),
    }
}

/// Builds a planned node with a stable structural parent.
///
/// - `id`: Durable node identity.
/// - `parent`: Structural parent relation.
///
/// # Returns
/// A ready plan node for projection assertions.
pub(super) fn node(id: EntityId, parent: Option<PlanNodeParent>) -> PlanNode {
    PlanNode {
        id,
        parent,
        title: format!("Work {id}"),
        purpose: "Keep plan semantics intact".to_owned(),
        expected_result: None,
        semantic_scope: Vec::new(),
        status: PlanNodeStatus::Ready,
        branch: None,
        result: None,
        superseded_by: None,
        abandon_reason: None,
        last_failure: None,
        assignment: None,
        claim: None,
        claim_generation: 0,
    }
}

/// Builds a node with one canonical direct status.
///
/// - `id`: Durable node identity.
/// - `parent`: Structural parent relation.
/// - `status`: Canonical `PlanNodeStatus` stored on the node.
///
/// # Returns
/// The node carrying that direct status.
pub(super) fn node_with_status(
    id: EntityId,
    parent: Option<PlanNodeParent>,
    status: PlanNodeStatus,
) -> PlanNode {
    let mut node = node(id, parent);
    node.status = status;
    node
}

/// Builds an abandoned node carrying one typed abandonment reason.
///
/// - `id`: Durable node identity.
/// - `parent`: Structural parent relation.
/// - `reason`: Typed reason retained on the canonical node.
///
/// # Returns
/// The abandoned node.
pub(super) fn abandoned_node(
    id: EntityId,
    parent: Option<PlanNodeParent>,
    reason: PlanAbandonReason,
) -> PlanNode {
    let mut node = node_with_status(id, parent, PlanNodeStatus::Abandoned);
    node.abandon_reason = Some(reason);
    node
}

/// Builds one completion gate requiring an exact structural child.
///
/// - `id`: Durable gate identity.
/// - `owner`: Node whose completion the gate constrains.
/// - `child`: Required structural child.
///
/// # Returns
/// The typed completion gate.
pub(super) fn required_child_gate(id: EntityId, owner: EntityId, child: EntityId) -> PlanGate {
    PlanGate {
        id,
        owner,
        phase: PlanGatePhase::Completion,
        requirement: PlanGateRequirement::RequiredChildCompleted { child },
        waiver: None,
    }
}

/// Builds one entry gate that requires a passing validation.
///
/// - `id`: Durable gate identity.
/// - `owner`: Node whose readiness the gate constrains.
///
/// # Returns
/// The typed entry gate.
pub(super) fn validation_entry_gate(id: EntityId, owner: EntityId) -> PlanGate {
    PlanGate {
        id,
        owner,
        phase: PlanGatePhase::Entry,
        requirement: PlanGateRequirement::ValidationPassed { profile: None },
        waiver: None,
    }
}

/// Builds one evidence association for a node or one of its gates.
///
/// - `id`: Durable evidence-association identity.
/// - `owner`: Node or gate receiving the evidence.
/// - `role`: Supports, contradicts, or informational role.
/// - `source`: Exact semantic entity referenced by the association.
///
/// # Returns
/// The evidence association with an exact reference and no copied payload.
pub(super) fn evidence(
    id: EntityId,
    owner: PlanEvidenceOwner,
    role: PlanEvidenceRole,
    source: EntityId,
) -> PlanEvidenceAssociation {
    PlanEvidenceAssociation {
        id,
        owner,
        role,
        evidence: PlanEvidenceReference {
            kind: PlanEvidenceKind::ValidationResult,
            reference: PlanSemanticReference {
                entity: source,
                revision: None,
                snapshot: None,
            },
            attributes: BTreeMap::default(),
        },
        note: None,
        actor: None,
    }
}

/// Builds one current summary link for a plan node.
///
/// - `id`: Durable summary identity.
/// - `node`: Summarized plan node.
/// - `text`: Concise current summary text.
///
/// # Returns
/// The summary link.
pub(super) fn summary(id: EntityId, node: EntityId, text: &str) -> PlanNodeSummary {
    PlanNodeSummary {
        id,
        node,
        text: text.to_owned(),
        sources: Vec::new(),
    }
}

/// Builds one failed evidence reference retained on a retried node.
///
/// - `source`: Exact failed result identity.
///
/// # Returns
/// The failure evidence reference carried by `PlanNodeView::last_failure`.
pub(super) fn failure_reference(source: EntityId) -> PlanEvidenceReference {
    PlanEvidenceReference {
        kind: PlanEvidenceKind::Failure,
        reference: PlanSemanticReference {
            entity: source,
            revision: None,
            snapshot: None,
        },
        attributes: BTreeMap::default(),
    }
}

/// Wraps durable plans in the canonical session snapshot the Plans view reads.
///
/// - `plans`: Durable plans projected by the Plans view.
///
/// # Returns
/// A session snapshot with no branches and the supplied plans.
pub(super) fn session_with_plans(plans: Vec<Plan>) -> Session {
    Session {
        id: EntityId::from_bytes([9; 16]),
        branches: Vec::new(),
        goals: Vec::new(),
        tasks: Vec::new(),
        plans,
        execution: scriberig_session::SessionExecutionSelection::default(),
    }
}

/// Wraps durable plans together with completed branches that carry results.
///
/// - `plans`: Durable plans projected by the Plans view.
/// - `results`: Result identities, each produced by a completed branch of the same identity.
///
/// # Returns
/// A session snapshot whose branch results feed the shared plan views.
pub(super) fn session_with_results(plans: Vec<Plan>, results: &[EntityId]) -> Session {
    let mut session = session_with_plans(plans);
    for result in results {
        let mut branch = Branch::new(*result, "Producer branch").expect("producer branch");
        branch.status = BranchStatus::Completed;
        branch.result = Some(BranchResult {
            id: *result,
            branch: *result,
            status: BranchStatus::Completed,
            summary: Some("Producer finished".to_owned()),
            result_kind: None,
            candidate_context: None,
            references: Vec::new(),
            result_handles: Vec::new(),
            touched: Vec::new(),
            produced_changes: Vec::new(),
            validations: Vec::new(),
            unresolved_questions: Vec::new(),
        });
        session.branches.push(branch);
    }
    session
}

/// Recomputes the shared node view for one node with the session's own results.
///
/// - `session`: Canonical session snapshot the projection reads.
/// - `plan`: Durable plan copy under test.
/// - `node`: Durable node identity.
///
/// # Returns
/// The shared `PlanNodeView`, which is the semantics the component must render.
pub(super) fn shared_node_view(session: &Session, plan: &Plan, node: EntityId) -> PlanNodeView {
    let results = session
        .branches
        .iter()
        .filter_map(|branch| branch.result.clone())
        .collect::<Vec<_>>();
    let rollups = plan.node_rollups().expect("shared rollups");
    plan.node_view(node, plan.source_snapshot, Some(&results), &rollups)
        .expect("shared node view")
}

/// Direct node status counts reported by the shared plan view.
///
/// - `counts`: Counts from `PlanView::status_counts`.
///
/// # Returns
/// The total number of plan nodes the shared view counts.
pub(super) fn status_total(counts: PlanNodeStatusCounts) -> u32 {
    counts
        .planned
        .saturating_add(counts.ready)
        .saturating_add(counts.running)
        .saturating_add(counts.completed)
        .saturating_add(counts.failed)
        .saturating_add(counts.abandoned)
}

/// Projects one plan copy and returns its rows with the snapshot behind them.
///
/// - `plan`: Durable plan copy under test.
///
/// # Returns
/// The projected rows and the canonical session snapshot they came from.
pub(super) fn plan_rows(plan: Plan) -> (Vec<super::PlanTreeRow>, Session) {
    let session = session_with_plans(vec![plan]);
    (super::rows_for_session(&session), session)
}

/// Builds the details Markdown for one stable plan or node selection.
///
/// - `session`: Canonical session snapshot behind the projected rows.
/// - `rows`: Stable projected rows.
/// - `plan`: Durable plan identity.
/// - `node`: Selected plan-node identity, or `None` for the plan header.
///
/// # Returns
/// Safe details Markdown for that selection.
pub(super) fn selected_details(
    session: &Session,
    rows: &[super::PlanTreeRow],
    plan: EntityId,
    node: Option<EntityId>,
) -> String {
    super::details::plan_details(session, Some(PlanSelection { plan, node }), rows)
}
