//! Formats typed gate definitions and shared evaluation results for plan nodes.

use std::fmt::Write as _;

use scriberig_core::EntityId;
use scriberig_session::{Plan, PlanGatePhase, PlanGateRequirement};

/// Appends current gate definitions and evaluated evidence references for a node.
///
/// - `output`: Markdown receiving safe plan details.
/// - `plan`: Durable plan snapshot containing gate definitions and evidence.
/// - `node`: Stable selected plan-node identity.
pub(super) fn append_node_gates(output: &mut String, plan: &Plan, node: EntityId) {
    let mut wrote_header = false;
    for phase in [PlanGatePhase::Entry, PlanGatePhase::Completion] {
        for evaluation in plan.evaluate_node_gates(node, phase) {
            let Some(gate) = plan.gates.iter().find(|gate| gate.id == evaluation.gate) else {
                continue;
            };
            if !wrote_header {
                super::super::super::common::append_value(output, "## Gate requirements");
                wrote_header = true;
            }
            super::super::super::common::append_value(
                output,
                &format!(
                    "{:?} gate {} · {:?} · {}",
                    phase,
                    gate.id,
                    evaluation.status,
                    requirement_summary(&gate.requirement)
                ),
            );
            if let Some(reason) = evaluation.reason {
                super::super::super::common::append_value(
                    output,
                    &format!("Gate status: {reason}"),
                );
            }
            for evidence in evaluation.supporting_evidence {
                let reference = evidence.reference;
                let mut line = format!("Gate evidence: {} · {:?}", reference.entity, evidence.kind);
                if let Some(revision) = reference.revision {
                    let _ = write!(line, " · revision {revision}");
                }
                if let Some(snapshot) = reference.snapshot {
                    let _ = write!(line, " · snapshot {snapshot}");
                }
                super::super::super::common::append_value(output, &line);
            }
        }
    }
}

/// Formats a typed gate requirement using readable semantic identities.
///
/// - `requirement`: Canonical plan requirement to describe.
///
/// A concise safe requirement description.
/// # Returns
/// A readable summary that contains no untrusted rendering controls.
fn requirement_summary(requirement: &PlanGateRequirement) -> String {
    match requirement {
        PlanGateRequirement::EvidencePresent { kind } => format!("evidence present: {kind:?}"),
        PlanGateRequirement::ValidationPassed { profile } => format!(
            "validation passed{}",
            profile
                .as_deref()
                .map_or_else(String::new, |profile| format!(" for {profile}"))
        ),
        PlanGateRequirement::DecisionResolved { decision } => {
            format!("decision resolved: {decision}")
        }
        PlanGateRequirement::HumanApproval { policy, role } => format!(
            "human approval: {policy}{}",
            role.as_deref()
                .map_or_else(String::new, |role| format!(" · role {role}"))
        ),
        PlanGateRequirement::SemanticCondition {
            reference,
            predicate,
        } => format!(
            "semantic condition: {predicate:?} · entity {}{}{}",
            reference.entity,
            reference
                .revision
                .map_or_else(String::new, |revision| format!(" · revision {revision}")),
            reference
                .snapshot
                .map_or_else(String::new, |snapshot| format!(" · snapshot {snapshot}"))
        ),
        PlanGateRequirement::RequiredChildCompleted { child } => {
            format!("required child completed: {child}")
        }
    }
}
