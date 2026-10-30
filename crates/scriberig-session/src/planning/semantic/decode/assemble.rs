//! Assembles one validated plan projection from semantic records.

use std::collections::{BTreeMap, BTreeSet};

use super::super::encode::RelationIdentity;
use super::{
    ATTEMPT_TYPE, DEPENDENCY_TYPE, DependencyIdentity, DependencyPayload, EVIDENCE_TYPE,
    EvidencePayload, GATE_TYPE, GatePayload, HISTORY_TYPE, HistoryIdentity, HistoryPayload,
    MEMBERSHIP_TYPE, MembershipPayload, NODE_TYPE, NodePayload, PLAN_TYPE, PlanPayload,
    PlanSemanticError, PlanSemanticSnapshot, STRUCTURE_TYPE, SUMMARY_LINK_TYPE, SUMMARY_TYPE,
    StructurePayload, SummaryLinkPayload, SummaryPayload, decode_payload, invalid, parent_identity,
    relation_id, validate_revision,
};
use crate::planning::{
    Plan, PlanDependency, PlanEvidenceAssociation, PlanEvidenceOwner, PlanGate,
    PlanGateRequirement, PlanNode, PlanNodeParent, PlanNodeSummary, PlanRevision,
};

#[expect(
    clippy::too_many_lines,
    reason = "One-pass decoding keeps exhaustive validation coupled to the semantic type dispatch."
)]
/// Assembles one typed plan projection after validating every object revision.
///
/// # Parameters
/// - `snapshot`: Current immutable plan object set.
/// - `history`: Historical compatibility records retained by the caller.
///
/// # Returns
/// The assembled plan projection.
///
/// # Errors
/// Returns an error for invalid, missing, duplicate, or unsupported records.
pub(in crate::planning::semantic) fn into_plan(
    snapshot: &PlanSemanticSnapshot,
    history: Vec<PlanRevision>,
) -> Result<Plan, PlanSemanticError> {
    super::attempts(snapshot)?;
    let mut plan_payload = None;
    let mut nodes = BTreeMap::new();
    let mut memberships = BTreeMap::new();
    let mut structures = BTreeMap::new();
    let mut dependencies = Vec::new();
    let mut history_metadata = BTreeMap::new();
    let mut summaries = BTreeMap::new();
    let mut summary_links = BTreeMap::new();
    let mut gates = Vec::new();
    let mut evidence = Vec::new();

    for object in &snapshot.objects {
        validate_revision(object)?;
        match object.object_type.as_str() {
            PLAN_TYPE => {
                if object.entity != snapshot.plan || plan_payload.is_some() {
                    return Err(invalid("duplicate or mismatched plan object"));
                }
                let payload: PlanPayload = decode_payload(object)?;
                if payload.schema_version != 1 {
                    return Err(invalid("unsupported plan payload version"));
                }
                plan_payload = Some(payload);
            }
            NODE_TYPE => {
                let payload: NodePayload = decode_payload(object)?;
                if payload.schema_version != 1
                    || payload.plan != snapshot.plan
                    || nodes.contains_key(&object.entity)
                {
                    return Err(invalid("duplicate, mismatched, or unsupported node object"));
                }
                nodes.insert(object.entity, payload);
            }
            MEMBERSHIP_TYPE => {
                let payload: MembershipPayload = decode_payload(object)?;
                if payload.schema_version != 1
                    || payload.plan != snapshot.plan
                    || relation_id(
                        MEMBERSHIP_TYPE,
                        &RelationIdentity {
                            owner: payload.plan,
                            target: payload.node,
                        },
                    )? != object.entity
                    || memberships.insert(payload.node, payload.ordinal).is_some()
                {
                    return Err(invalid("duplicate, mismatched, or unsupported membership"));
                }
            }
            STRUCTURE_TYPE => {
                let payload: StructurePayload = decode_payload(object)?;
                if payload.schema_version != 1
                    || payload.plan != snapshot.plan
                    || relation_id(
                        STRUCTURE_TYPE,
                        &RelationIdentity {
                            owner: payload.child,
                            target: payload.parent.map_or(payload.root_branch, parent_identity),
                        },
                    )? != object.entity
                    || structures
                        .insert(payload.child, (payload.ordinal, payload.parent))
                        .is_some()
                {
                    return Err(invalid("duplicate or unsupported structure relation"));
                }
            }
            DEPENDENCY_TYPE => {
                let payload: DependencyPayload = decode_payload(object)?;
                if payload.schema_version != 1
                    || payload.plan != snapshot.plan
                    || relation_id(
                        DEPENDENCY_TYPE,
                        &DependencyIdentity {
                            plan: payload.plan,
                            prerequisite: payload.prerequisite,
                            dependent: payload.dependent,
                        },
                    )? != object.entity
                {
                    return Err(invalid("mismatched or unsupported dependency"));
                }
                dependencies.push(PlanDependency {
                    prerequisite: payload.prerequisite,
                    dependent: payload.dependent,
                    fulfilled_by: payload.fulfilled_by,
                });
            }
            HISTORY_TYPE => {
                let payload: HistoryPayload = decode_payload(object)?;
                if payload.schema_version != 1
                    || payload.plan != snapshot.plan
                    || relation_id(
                        HISTORY_TYPE,
                        &HistoryIdentity {
                            plan: payload.plan,
                            mutation_revision: payload.mutation_revision,
                        },
                    )? != object.entity
                    || history_metadata
                        .insert(payload.mutation_revision, payload.reason)
                        .is_some()
                {
                    return Err(invalid(
                        "duplicate, mismatched, or unsupported history record",
                    ));
                }
            }
            SUMMARY_TYPE => {
                let payload: SummaryPayload = decode_payload(object)?;
                if payload.schema_version != 1
                    || payload.plan != snapshot.plan
                    || summaries
                        .insert(
                            object.entity,
                            PlanNodeSummary {
                                id: object.entity,
                                node: payload.node,
                                text: payload.text,
                                sources: payload.sources,
                            },
                        )
                        .is_some()
                {
                    return Err(invalid("duplicate, mismatched, or unsupported summary"));
                }
            }
            SUMMARY_LINK_TYPE => {
                let payload: SummaryLinkPayload = decode_payload(object)?;
                if payload.schema_version != 1
                    || payload.plan != snapshot.plan
                    || relation_id(
                        SUMMARY_LINK_TYPE,
                        &RelationIdentity {
                            owner: payload.node,
                            target: payload.summary,
                        },
                    )? != object.entity
                    || summary_links
                        .insert(payload.node, payload.summary)
                        .is_some()
                {
                    return Err(invalid(
                        "duplicate, mismatched, or unsupported summary link",
                    ));
                }
            }
            GATE_TYPE => {
                let payload: GatePayload = decode_payload(object)?;
                if payload.schema_version != 1 || payload.plan != snapshot.plan {
                    return Err(invalid("mismatched or unsupported gate"));
                }
                gates.push(PlanGate {
                    id: object.entity,
                    owner: payload.owner,
                    phase: payload.phase,
                    requirement: payload.requirement,
                    waiver: payload.waiver,
                });
            }
            EVIDENCE_TYPE => {
                let payload: EvidencePayload = decode_payload(object)?;
                if payload.schema_version != 1 || payload.plan != snapshot.plan {
                    return Err(invalid("mismatched or unsupported evidence"));
                }
                evidence.push(PlanEvidenceAssociation {
                    id: object.entity,
                    owner: payload.owner,
                    role: payload.role,
                    evidence: payload.evidence,
                    note: payload.note,
                    actor: payload.actor,
                });
            }
            ATTEMPT_TYPE => {}
            type_name => {
                return Err(invalid(format!(
                    "unsupported semantic object type {type_name}"
                )));
            }
        }
    }

    let payload = plan_payload.ok_or_else(|| invalid("missing plan object"))?;
    if nodes.len() != memberships.len() || nodes.len() != structures.len() {
        return Err(invalid("node, membership, and structure sets differ"));
    }
    let mut ordered_nodes = memberships.into_iter().collect::<Vec<_>>();
    ordered_nodes.sort_by_key(|(node, ordinal)| (*ordinal, *node));
    let mut plan_nodes = Vec::with_capacity(ordered_nodes.len());
    let mut root_nodes = Vec::new();
    let mut ordinals = BTreeSet::new();
    for (node_id, ordinal) in ordered_nodes {
        if !ordinals.insert(ordinal) {
            return Err(invalid("duplicate plan membership ordinal"));
        }
        let node = nodes
            .remove(&node_id)
            .ok_or_else(|| invalid("membership points to a missing node"))?;
        let (structure_ordinal, parent) = structures
            .remove(&node_id)
            .ok_or_else(|| invalid("node has no structural relation"))?;
        if structure_ordinal != ordinal {
            return Err(invalid("membership and structure order disagree"));
        }
        if matches!(parent, Some(PlanNodeParent::Branch(_)) | None) {
            root_nodes.push(node_id);
        }
        plan_nodes.push(PlanNode {
            id: node_id,
            parent,
            title: node.title,
            purpose: node.purpose,
            expected_result: node.expected_result,
            semantic_scope: node.semantic_scope,
            status: node.status,
            branch: node.branch,
            result: node.result,
            superseded_by: node.superseded_by,
            abandon_reason: node.abandon_reason,
            last_failure: node.last_failure,
            assignment: node.assignment,
            claim: node.claim,
            claim_generation: node.claim_generation,
        });
    }
    if !nodes.is_empty() || !structures.is_empty() {
        return Err(invalid("unreferenced node or structure relation"));
    }
    dependencies.sort_by_key(|edge| (edge.prerequisite, edge.dependent));
    if summary_links.len() != summaries.len() {
        return Err(invalid("summary and summary-link sets differ"));
    }
    let mut ordered_summaries = Vec::with_capacity(summary_links.len());
    for (node_id, summary_id) in summary_links {
        let summary = summaries
            .remove(&summary_id)
            .ok_or_else(|| invalid("summary link points to a missing summary"))?;
        if summary.node != node_id || !plan_nodes.iter().any(|node| node.id == node_id) {
            return Err(invalid("summary link has a mismatched node"));
        }
        ordered_summaries.push(summary);
    }
    if !summaries.is_empty() {
        return Err(invalid("unreferenced summary object"));
    }
    if gates.iter().any(|gate| {
        !plan_nodes.iter().any(|node| node.id == gate.owner)
            || matches!(
                &gate.requirement,
                PlanGateRequirement::RequiredChildCompleted { child }
                    if !plan_nodes.iter().any(|node| {
                        node.id == *child
                            && node.parent == Some(PlanNodeParent::Node(gate.owner))
                    })
            )
    }) {
        return Err(invalid("gate owner or required child is missing"));
    }
    if evidence.iter().any(|association| match association.owner {
        PlanEvidenceOwner::Node(node) => !plan_nodes.iter().any(|item| item.id == node),
        PlanEvidenceOwner::Gate(gate) => !gates.iter().any(|item| item.id == gate),
    }) {
        return Err(invalid("evidence owner is missing"));
    }
    let mut plan = Plan {
        id: snapshot.plan,
        source_snapshot: snapshot.snapshot,
        session: payload.session,
        anchor: payload.anchor,
        goal: payload.goal,
        revision: payload.mutation_revision,
        status: payload.status,
        root_branch: payload.root_branch,
        nodes: plan_nodes,
        dependencies,
        summaries: ordered_summaries,
        evidence,
        gates,
        coordinator: payload.coordinator,
        coordinator_cursor: payload.coordinator_cursor,
        coordinator_claim: payload.coordinator_claim,
        coordinator_generation: payload.coordinator_generation,
        root_nodes,
        history,
    };
    for record in &mut plan.history {
        if let Some(reason) = history_metadata.get(&record.revision) {
            record.reason = reason.clone();
        }
    }
    Ok(plan)
}
