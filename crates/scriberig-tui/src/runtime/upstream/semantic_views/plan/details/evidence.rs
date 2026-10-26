//! Presents exact plan evidence references with shared inspection commands.

use std::fmt::Write as _;

use scriberig_core::EntityId;
use scriberig_session::{Plan, PlanEvidenceOwner};

/// Appends evidence owned by a plan node or one of its gates.
///
/// References with an exact snapshot include an `/inspect` command bound to
/// that snapshot. Other references retain their identities and disclose when
/// the shared inspection API cannot resolve their exact historical context.
///
/// - `output`: Markdown receiving safe plan details.
/// - `plan`: Durable plan snapshot containing evidence associations.
/// - `node`: Stable selected plan-node identity.
pub(super) fn append_node_evidence(output: &mut String, plan: &Plan, node: EntityId) {
    let gate_ids = plan
        .gates
        .iter()
        .filter(|gate| gate.owner == node)
        .map(|gate| gate.id)
        .collect::<Vec<_>>();
    let evidence = plan
        .evidence
        .iter()
        .filter(|association| match association.owner {
            PlanEvidenceOwner::Node(owner) => owner == node,
            PlanEvidenceOwner::Gate(gate) => gate_ids.contains(&gate),
        });
    for association in evidence {
        let reference = association.evidence.reference;
        let mut line = format!(
            "Evidence: {} · {:?} · {:?}",
            reference.entity, association.evidence.kind, association.role
        );
        if let Some(revision) = reference.revision {
            let _ = write!(line, " · revision {revision}");
        }
        if let Some(snapshot) = reference.snapshot {
            let _ = write!(
                line,
                " · snapshot {snapshot} · inspect `/inspect {} --snapshot {snapshot}`",
                reference.entity
            );
        } else if reference.revision.is_none() {
            let _ = write!(line, " · inspect `/inspect {}`", reference.entity);
        } else {
            line.push_str(" · exact snapshot unavailable for shared inspection");
        }
        super::super::super::common::append_value(output, &line);
    }
}
