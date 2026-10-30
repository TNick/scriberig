//! Projects durable plan and dependency details into upstream Markdown.

use crate::selection::PlanSelection;
use scriberig_session::{Plan, PlanGatePhase, PlanNodeParent, PlanNodeView, Session};

/// Builds bounded details for a plan and its explicit dependency graph.
///
/// # Parameters
/// - `session`: Canonical session snapshot supplying the plans.
/// - `cursor`: Stable selected plan or node identity.
/// - `rows`: Stable projected plan/node rows.
///
/// # Returns
/// Safe Markdown details for the selected plan or node.
pub(super) fn plan_details(
    session: &Session,
    cursor: Option<PlanSelection>,
    rows: &[super::PlanTreeRow],
) -> String {
    let selection = cursor
        .filter(|selection| rows.iter().any(|row| row.selection == *selection))
        .or_else(|| rows.first().map(|row| row.selection));
    let Some(selection) = selection else {
        return "# Plans\n\nNo durable plans are available.".to_owned();
    };
    let Some(row) = rows.iter().find(|row| row.selection == selection) else {
        return "# Plans\n\nSelected plan is no longer available.".to_owned();
    };
    let Some(plan) = session.plans.iter().find(|plan| plan.id == selection.plan) else {
        return "# Plans\n\nSelected plan is no longer available.".to_owned();
    };
    let mut output = String::from("# Plan\n\n");
    super::super::common::append_value(&mut output, &format!("Identity: {}", row.plan.id));
    super::super::common::append_value(
        &mut output,
        &format!("Mutation revision: {}", row.plan.mutation_revision),
    );
    super::super::common::append_value(
        &mut output,
        &format!("Exact revision: {}", row.plan.revision),
    );
    if let Some(snapshot) = row.plan.snapshot {
        super::super::common::append_value(&mut output, &format!("Snapshot: {snapshot}"));
    }
    super::status::append_plan_status(&mut output, row.plan.status);
    super::super::common::append_value(&mut output, &format!("Goal: {}", row.plan.goal));
    if let Some(claim) = &plan.coordinator_claim {
        super::super::common::append_value(
            &mut output,
            &format!("Coordinator claim: {} · {}", claim.id, claim.owner),
        );
        if let Some(operation) = &claim.operation {
            super::super::common::append_value(
                &mut output,
                &format!("Coordinator operation: {operation}"),
            );
        }
    }
    if let Some(cursor) = row.plan.coordinator_cursor {
        super::super::common::append_value(&mut output, &format!("Coordinator node: {cursor}"));
    }
    if let Some(coordinator) = row.plan.coordinator {
        super::super::common::append_value(
            &mut output,
            &format!("Coordinator branch: {coordinator}"),
        );
    }
    append_plan_history(&mut output, plan);
    if let Some(node) = row.node.as_ref() {
        append_node_details(&mut output, plan, node);
    }
    super::super::common::append_value(&mut output, "## Explicit dependency edges");
    for edge in plan
        .dependencies
        .iter()
        .take(super::super::common::MAX_ROWS)
    {
        super::super::common::append_value(
            &mut output,
            &format!(
                "{} → {} · result {}",
                edge.prerequisite,
                edge.dependent,
                edge.fulfilled_by
                    .map_or_else(|| "pending".to_owned(), |id| id.to_string())
            ),
        );
    }
    output
}

/// Projects bounded immutable replan history into selected plan details.
///
/// # Parameters
/// - `output`: Markdown rendered by upstream `rpi_tui::Markdown`.
/// - `plan`: Canonical durable `ScribeRig` plan with prior revisions.
fn append_plan_history(output: &mut String, plan: &Plan) {
    if plan.history.is_empty() {
        return;
    }
    super::super::common::append_value(output, "## Prior plan revisions");
    for revision in plan.history.iter().rev().take(20).rev() {
        super::super::common::append_value(
            output,
            &format!(
                "Revision {} · {:?} · {} nodes · {} dependencies",
                revision.revision,
                revision.status,
                revision.nodes.len(),
                revision.dependencies.len()
            ),
        );
        super::super::common::append_value(output, &format!("Goal: {}", revision.goal));
        if let Some(reason) = &revision.reason {
            super::super::common::append_value(output, &format!("Replan reason: {reason}"));
        }
    }
    if plan.history.len() > 20 {
        super::super::common::append_value(
            output,
            &format!("{} earlier revisions omitted.", plan.history.len() - 20),
        );
    }
}

/// Appends durable execution and structural details for one plan node.
///
/// # Parameters
/// - `output`: Markdown receiving safe plan details.
/// - `plan`: Canonical plan supplying gate, evidence, and attempt details.
/// - `view`: Shared semantic view for the selected node.
fn append_node_details(output: &mut String, plan: &Plan, view: &PlanNodeView) {
    super::super::common::append_value(output, &format!("## Node {}", view.id));
    super::super::common::append_value(output, &format!("Title: {}", view.title));
    super::super::common::append_value(output, &format!("Purpose: {}", view.purpose));
    super::status::append_node_status(output, "Direct status", view.direct_status);
    super::status::append_node_status(output, "Effective status", view.effective_status);
    super::super::common::append_value(
        output,
        &format!("Contains ignored: {}", view.contains_ignored),
    );
    super::super::common::append_value(
        output,
        &format!("Contains abandoned: {}", view.contains_abandoned),
    );
    if let Some(summary) = &view.summary_preview {
        super::super::common::append_value(output, &format!("Summary: {summary}"));
    }
    super::super::common::append_value(
        output,
        &format!("Dependency blockers: {:?}", view.dependency_blockers),
    );
    super::super::common::append_value(output, &format!("Entry gates: {:?}", view.entry_gates));
    super::super::common::append_value(
        output,
        &format!("Completion gates: {:?}", view.completion_gates),
    );
    super::super::common::append_value(output, &format!("Evidence count: {}", view.evidence_count));
    super::super::common::append_value(output, &format!("Can retry: {}", view.can_retry));
    if let Some(failure) = &view.last_failure {
        super::super::common::append_value(
            output,
            &format!("Last failure: {}", evidence_reference(failure)),
        );
    }
    let Some(node) = plan.nodes.iter().find(|node| node.id == view.id) else {
        return;
    };
    match view.parent {
        Some(PlanNodeParent::Branch(branch)) => {
            super::super::common::append_value(
                output,
                &format!("Structural parent branch: {branch}"),
            );
        }
        Some(PlanNodeParent::Node(parent)) => {
            super::super::common::append_value(
                output,
                &format!("Structural parent node: {parent}"),
            );
        }
        None => super::super::common::append_value(output, "Structural parent: none"),
    }
    if let Some(branch) = view.branch {
        super::super::common::append_value(output, &format!("Execution branch: {branch}"));
    }
    if let Some(result) = view.result {
        super::super::common::append_value(output, &format!("BranchResult: {result}"));
    }
    if let Some(replacement) = node.superseded_by {
        super::super::common::append_value(output, &format!("Superseded by node: {replacement}"));
    }
    if let Some(assignment) = &view.assignment {
        super::super::common::append_value(
            output,
            &format!(
                "Assignment: {} / {} / {}",
                assignment.provider_id,
                assignment.model_id,
                assignment
                    .thinking_level
                    .as_deref()
                    .unwrap_or("unspecified")
            ),
        );
    }
    if let Some(claim) = &node.claim {
        super::super::common::append_value(
            output,
            &format!("Worker claim: {} · {}", claim.id, claim.owner),
        );
    }
    if let Some(expected) = &node.expected_result {
        super::super::common::append_value(output, &format!("Expected result: {}", expected.kind));
    }
    append_gate_details(output, plan, node.id);
    append_evidence_details(output, plan, node.id);
    append_attempt_details(output, plan, node.id);
}

/// Lists bounded gate states and reasons from the shared evaluator.
///
/// - `output`: Markdown receiving safe gate detail.
/// - `plan`: Canonical plan whose gates are evaluated by session semantics.
/// - `node`: Stable owning node identity.
fn append_gate_details(output: &mut String, plan: &Plan, node: scriberig_core::EntityId) {
    super::super::common::append_value(output, "### Gate details");
    for phase in [PlanGatePhase::Entry, PlanGatePhase::Completion] {
        for evaluation in plan
            .evaluate_node_gates(node, phase)
            .into_iter()
            .take(super::super::common::MAX_ROWS)
        {
            let requirement = plan
                .gates
                .iter()
                .find(|gate| gate.id == evaluation.gate)
                .map_or_else(
                    || "unknown requirement".to_owned(),
                    |gate| format!("{:?}", gate.requirement),
                );
            super::super::common::append_value(
                output,
                &format!(
                    "{phase:?} gate {} · {:?} · {requirement}",
                    evaluation.gate, evaluation.status
                ),
            );
            if let Some(reason) = evaluation.reason {
                super::super::common::append_value(output, &format!("Reason: {reason}"));
            }
        }
    }
}

/// Lists exact evidence references attached to a node or its gates.
///
/// - `output`: Markdown receiving safe evidence references.
/// - `plan`: Canonical plan containing evidence associations.
/// - `node`: Stable owning node identity.
fn append_evidence_details(output: &mut String, plan: &Plan, node: scriberig_core::EntityId) {
    super::super::common::append_value(output, "### Evidence references");
    super::super::common::append_value(
        output,
        "Inspect a source with /inspect <entity-id> [snapshot-id].",
    );
    let gate_ids = plan
        .gates
        .iter()
        .filter(|gate| gate.owner == node)
        .map(|gate| gate.id)
        .collect::<std::collections::BTreeSet<_>>();
    for association in plan
        .evidence
        .iter()
        .filter(|item| match item.owner {
            scriberig_session::PlanEvidenceOwner::Node(owner) => owner == node,
            scriberig_session::PlanEvidenceOwner::Gate(gate) => gate_ids.contains(&gate),
        })
        .take(super::super::common::MAX_ROWS)
    {
        let reference = &association.evidence.reference;
        super::super::common::append_value(
            output,
            &format!(
                "{} · {:?} · {:?} · entity {} · revision {} · snapshot {}",
                association.id,
                association.role,
                association.evidence.kind,
                reference.entity,
                reference
                    .revision
                    .map_or_else(|| "unspecified".to_owned(), |value| value.to_string()),
                reference
                    .snapshot
                    .map_or_else(|| "unspecified".to_owned(), |value| value.to_string())
            ),
        );
        if let Some(note) = &association.note {
            super::super::common::append_value(output, &format!("Note: {note}"));
        }
    }
}

/// Lists durable attempt summaries without loading worker transcripts.
///
/// - `output`: Markdown receiving attempt identity and outcome.
/// - `plan`: Canonical plan from which attempt history is derived.
/// - `node`: Stable node identity whose attempts are shown.
fn append_attempt_details(output: &mut String, plan: &Plan, node: scriberig_core::EntityId) {
    super::super::common::append_value(output, "### Attempts");
    if let Some(attempts) = plan.attempt_history(node) {
        for attempt in attempts
            .into_iter()
            .rev()
            .take(super::super::common::MAX_ROWS)
        {
            super::super::common::append_value(
                output,
                &format!(
                    "Generation {} · {:?} · branch {} · claim {} · result {}",
                    attempt.claim_generation,
                    attempt.status,
                    attempt.branch,
                    attempt
                        .claim
                        .map_or_else(|| "unspecified".to_owned(), |id| id.to_string()),
                    attempt
                        .result
                        .map_or_else(|| "none".to_owned(), |id| id.to_string())
                ),
            );
            if let Some(failure) = attempt.failure {
                super::super::common::append_value(
                    output,
                    &format!("Failure evidence: {}", evidence_reference(&failure)),
                );
            }
        }
    }
}

/// Formats an evidence source with exact revision and snapshot context.
///
/// - `evidence`: Typed evidence reference to show in the inspection view.
///
/// A safe identity-only description that can be opened with `/inspect`.
/// # Returns
/// Evidence kind, entity, and exact context identifiers.
fn evidence_reference(evidence: &scriberig_session::PlanEvidenceReference) -> String {
    let reference = evidence.reference;
    format!(
        "{:?} entity {} revision {} snapshot {}",
        evidence.kind,
        reference.entity,
        reference
            .revision
            .map_or_else(|| "unspecified".to_owned(), |value| value.to_string()),
        reference
            .snapshot
            .map_or_else(|| "unspecified".to_owned(), |value| value.to_string())
    )
}
