//! Builds typed immutable object payloads from the current plan projection.

use scriberig_core::EntityId;

use super::super::Plan;
use super::{PlanSemanticError, PlanSemanticSnapshot};

#[path = "encode/details.rs"]
mod details;
#[path = "encode/helpers.rs"]
mod helpers;
pub(super) use helpers::{parent_identity, push_object, push_relation, relation_id};

pub(super) const PLAN_TYPE: &str = crate::schema::SessionEntityType::Plan.as_str();
pub(super) const NODE_TYPE: &str = crate::schema::SessionEntityType::PlanNode.as_str();
pub(super) const MEMBERSHIP_TYPE: &str = crate::schema::SessionEntityType::PlanMembership.as_str();
pub(super) const STRUCTURE_TYPE: &str = crate::schema::SessionEntityType::PlanStructure.as_str();
pub(super) const DEPENDENCY_TYPE: &str = crate::schema::SessionEntityType::PlanDependency.as_str();
pub(super) const HISTORY_TYPE: &str = crate::schema::SessionEntityType::PlanRevision.as_str();
pub(super) const GATE_TYPE: &str = crate::schema::SessionEntityType::PlanGate.as_str();
pub(super) const EVIDENCE_TYPE: &str = crate::schema::SessionEntityType::PlanEvidence.as_str();
pub(super) const SUMMARY_TYPE: &str = crate::schema::SessionEntityType::PlanSummary.as_str();
pub(super) const SUMMARY_LINK_TYPE: &str =
    crate::schema::SessionEntityType::PlanSummaryLink.as_str();
pub(super) const ATTEMPT_TYPE: &str = crate::schema::SessionEntityType::PlanAttempt.as_str();

/// Encodes stable plan-level lifecycle and coordinator fields.
#[derive(serde::Serialize)]
struct PlanPayload<'a> {
    schema_version: u16,
    session: Option<EntityId>,
    anchor: Option<&'a crate::ContinuationAnchor>,
    goal: &'a str,
    mutation_revision: u64,
    status: super::super::PlanStatus,
    root_branch: EntityId,
    coordinator: Option<EntityId>,
    coordinator_cursor: Option<EntityId>,
    coordinator_claim: Option<&'a super::super::CoordinatorClaim>,
    coordinator_generation: u64,
}

/// Encodes one independently revisioned plan node.
#[derive(serde::Serialize)]
struct NodePayload<'a> {
    schema_version: u16,
    plan: EntityId,
    title: &'a str,
    purpose: &'a str,
    expected_result: &'a Option<super::super::ExpectedResult>,
    semantic_scope: &'a [String],
    status: super::super::PlanNodeStatus,
    branch: Option<EntityId>,
    result: Option<EntityId>,
    superseded_by: Option<EntityId>,
    abandon_reason: Option<super::super::PlanAbandonReason>,
    last_failure: &'a Option<super::super::PlanEvidenceReference>,
    assignment: &'a Option<super::super::ModelAssignment>,
    claim: &'a Option<super::super::WorkerClaim>,
    claim_generation: u64,
}

/// Encodes plan membership and stable node ordering.
#[derive(serde::Serialize)]
struct MembershipPayload {
    schema_version: u16,
    plan: EntityId,
    node: EntityId,
    ordinal: u64,
}

/// Encodes the independent structural parent relation.
#[derive(serde::Serialize)]
struct StructurePayload {
    schema_version: u16,
    plan: EntityId,
    root_branch: EntityId,
    child: EntityId,
    parent: Option<super::super::PlanNodeParent>,
    ordinal: u64,
}

/// Encodes a schedulable prerequisite dependency.
#[derive(serde::Serialize)]
struct DependencyPayload {
    schema_version: u16,
    plan: EntityId,
    prerequisite: EntityId,
    dependent: EntityId,
    fulfilled_by: Option<EntityId>,
}

/// Provides stable owner and target identity for a relation entity.
#[derive(serde::Serialize)]
pub(super) struct RelationIdentity {
    pub(super) owner: EntityId,
    pub(super) target: EntityId,
}

/// Provides a stable plan-scoped dependency identity.
#[derive(serde::Serialize)]
pub(super) struct DependencyIdentity {
    pub(super) plan: EntityId,
    pub(super) prerequisite: EntityId,
    pub(super) dependent: EntityId,
}

/// Encodes safe history metadata for one plan mutation revision.
#[derive(serde::Serialize)]
struct HistoryPayload<'a> {
    schema_version: u16,
    plan: EntityId,
    mutation_revision: u64,
    reason: &'a Option<String>,
}

/// Provides stable identity for one plan history record.
#[derive(serde::Serialize)]
struct HistoryIdentity {
    plan: EntityId,
    mutation_revision: u64,
}

/// Encodes a bounded node summary and exact semantic sources.
#[derive(serde::Serialize)]
struct SummaryPayload<'a> {
    schema_version: u16,
    plan: EntityId,
    node: EntityId,
    text: &'a str,
    sources: &'a [super::super::PlanSemanticReference],
}

/// Encodes the relation from a plan node to its current summary.
#[derive(serde::Serialize)]
struct SummaryLinkPayload {
    schema_version: u16,
    plan: EntityId,
    node: EntityId,
    summary: EntityId,
}

/// Encodes one typed gate and its optional waiver provenance.
#[derive(serde::Serialize)]
struct GatePayload<'a> {
    schema_version: u16,
    plan: EntityId,
    owner: EntityId,
    phase: super::super::PlanGatePhase,
    requirement: &'a super::super::PlanGateRequirement,
    waiver: &'a Option<super::super::PlanGateWaiver>,
}

/// Encodes a typed evidence association without source payload duplication.
#[derive(serde::Serialize)]
struct EvidencePayload<'a> {
    schema_version: u16,
    plan: EntityId,
    owner: super::super::PlanEvidenceOwner,
    role: super::super::PlanEvidenceRole,
    evidence: &'a super::super::PlanEvidenceReference,
    note: &'a Option<String>,
    actor: Option<EntityId>,
}

/// Encodes one isolated worker attempt independently from its plan node.
#[derive(serde::Serialize)]
struct AttemptPayload<'a> {
    schema_version: u16,
    plan: EntityId,
    node: EntityId,
    branch: EntityId,
    claim_generation: u64,
    claim: Option<EntityId>,
    status: super::super::PlanNodeStatus,
    result: Option<EntityId>,
    failure: &'a Option<super::super::PlanEvidenceReference>,
}

/// Provides stable identity for one plan-node execution generation.
#[derive(serde::Serialize)]
struct AttemptIdentity {
    plan: EntityId,
    node: EntityId,
    branch: EntityId,
    claim_generation: u64,
}

/// Encodes a plan view into current independently revisioned objects.
///
/// # Parameters
/// - `plan`: Aggregate read model to split into semantic records.
///
/// # Returns
/// A stable object set sorted by entity and type identity.
///
/// # Errors
/// Returns a serialization, canonical identity, or graph validation error.
pub(super) fn from_plan(plan: &Plan) -> Result<PlanSemanticSnapshot, PlanSemanticError> {
    plan.node_rollups().map_err(|error| {
        PlanSemanticError::InvalidObject(format!("invalid plan structure: {error:?}"))
    })?;
    let mut objects = Vec::new();
    push_object(
        &mut objects,
        plan.id,
        PLAN_TYPE,
        &PlanPayload {
            schema_version: 1,
            session: plan.session,
            anchor: plan.anchor.as_ref(),
            goal: &plan.goal,
            mutation_revision: plan.revision,
            status: plan.status,
            root_branch: plan.root_branch,
            coordinator: plan.coordinator,
            coordinator_cursor: plan.coordinator_cursor,
            coordinator_claim: plan.coordinator_claim.as_ref(),
            coordinator_generation: plan.coordinator_generation,
        },
    )?;

    for (ordinal, node) in plan.nodes.iter().enumerate() {
        push_object(
            &mut objects,
            node.id,
            NODE_TYPE,
            &NodePayload {
                schema_version: 1,
                plan: plan.id,
                title: &node.title,
                purpose: &node.purpose,
                expected_result: &node.expected_result,
                semantic_scope: &node.semantic_scope,
                status: node.status,
                branch: node.branch,
                result: node.result,
                superseded_by: node.superseded_by,
                abandon_reason: node.abandon_reason,
                last_failure: &node.last_failure,
                assignment: &node.assignment,
                claim: &node.claim,
                claim_generation: node.claim_generation,
            },
        )?;
        let ordinal = u64::try_from(ordinal).map_err(|error| {
            PlanSemanticError::Serialization(format!("invalid node ordinal: {error}"))
        })?;
        let membership = MembershipPayload {
            schema_version: 1,
            plan: plan.id,
            node: node.id,
            ordinal,
        };
        push_relation(
            &mut objects,
            MEMBERSHIP_TYPE,
            &RelationIdentity {
                owner: plan.id,
                target: node.id,
            },
            &membership,
        )?;
        let structure = StructurePayload {
            schema_version: 1,
            plan: plan.id,
            root_branch: plan.root_branch,
            child: node.id,
            parent: node.parent,
            ordinal,
        };
        push_relation(
            &mut objects,
            STRUCTURE_TYPE,
            &RelationIdentity {
                owner: node.id,
                target: node.parent.map_or(plan.root_branch, parent_identity),
            },
            &structure,
        )?;
    }

    details::append_related(plan, &mut objects)?;

    objects.sort_by(|left, right| {
        (&left.entity, left.object_type.as_str()).cmp(&(&right.entity, right.object_type.as_str()))
    });
    if objects
        .windows(2)
        .any(|pair| pair[0].entity == pair[1].entity)
    {
        return Err(PlanSemanticError::InvalidObject(
            "multiple plan semantic objects use one entity identity".to_owned(),
        ));
    }
    Ok(PlanSemanticSnapshot {
        plan: plan.id,
        snapshot: None,
        objects,
    })
}
