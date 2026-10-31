//! Defines typed payloads used by the semantic plan encoder.

use scriberig_core::EntityId;
use serde::Serialize;

#[derive(Serialize)]
/// Represents the immutable plan projection payload.
pub(super) struct PlanPayload<'a> {
    pub(super) schema_version: u16,
    pub(super) session: Option<EntityId>,
    pub(super) anchor: Option<&'a crate::ContinuationAnchor>,
    pub(super) goal: &'a str,
    pub(super) mutation_revision: u64,
    pub(super) status: super::super::super::PlanStatus,
    pub(super) root_branch: EntityId,
    pub(super) coordinator: Option<EntityId>,
    pub(super) coordinator_cursor: Option<EntityId>,
    pub(super) coordinator_claim: Option<&'a super::super::super::CoordinatorClaim>,
    pub(super) coordinator_generation: u64,
}
/// Converts or evaluates `NodePayload`.

#[derive(Serialize)]
pub(super) struct NodePayload<'a> {
    pub(super) schema_version: u16,
    pub(super) plan: EntityId,
    pub(super) title: &'a str,
    pub(super) purpose: &'a str,
    pub(super) expected_result: &'a Option<super::super::super::ExpectedResult>,
    pub(super) semantic_scope: &'a [String],
    pub(super) status: super::super::super::PlanNodeStatus,
    pub(super) branch: Option<EntityId>,
    pub(super) result: Option<EntityId>,
    pub(super) superseded_by: Option<EntityId>,
    pub(super) abandon_reason: Option<super::super::super::PlanAbandonReason>,
    pub(super) last_failure: &'a Option<super::super::super::PlanEvidenceReference>,
    pub(super) assignment: &'a Option<super::super::super::ModelAssignment>,
    pub(super) claim: &'a Option<super::super::super::WorkerClaim>,
    pub(super) claim_generation: u64,
}
/// Converts or evaluates `MembershipPayload`.

#[derive(Serialize)]
pub(super) struct MembershipPayload {
    pub(super) schema_version: u16,
    pub(super) plan: EntityId,
    pub(super) node: EntityId,
    pub(super) ordinal: u64,
}
/// Converts or evaluates `StructurePayload`.

#[derive(Serialize)]
pub(super) struct StructurePayload {
    pub(super) schema_version: u16,
    pub(super) plan: EntityId,
    pub(super) root_branch: EntityId,
    pub(super) child: EntityId,
    pub(super) parent: Option<super::super::super::PlanNodeParent>,
    pub(super) ordinal: u64,
    pub(super) root_ordinal: Option<u64>,
}
/// Converts or evaluates `DependencyPayload`.

#[derive(Serialize)]
pub(super) struct DependencyPayload {
    pub(super) schema_version: u16,
    pub(super) plan: EntityId,
    pub(super) prerequisite: EntityId,
    pub(super) dependent: EntityId,
    pub(super) fulfilled_by: Option<EntityId>,
}
/// Converts or evaluates `RelationIdentity`.

#[derive(Serialize)]
pub(in crate::planning::semantic) struct RelationIdentity {
    pub(in crate::planning::semantic) owner: EntityId,
    pub(in crate::planning::semantic) target: EntityId,
}
/// Converts or evaluates `DependencyIdentity`.

#[derive(Serialize)]
pub(in crate::planning::semantic) struct DependencyIdentity {
    pub(in crate::planning::semantic) plan: EntityId,
    pub(in crate::planning::semantic) prerequisite: EntityId,
    pub(in crate::planning::semantic) dependent: EntityId,
}
/// Converts or evaluates `HistoryPayload`.

#[derive(Serialize)]
pub(super) struct HistoryPayload<'a> {
    pub(super) schema_version: u16,
    pub(super) plan: EntityId,
    pub(super) mutation_revision: u64,
    pub(super) reason: &'a Option<String>,
}
/// Converts or evaluates `HistoryIdentity`.

#[derive(Serialize)]
pub(super) struct HistoryIdentity {
    pub(super) plan: EntityId,
    pub(super) mutation_revision: u64,
}
/// Converts or evaluates `SummaryPayload`.

#[derive(Serialize)]
pub(super) struct SummaryPayload<'a> {
    pub(super) schema_version: u16,
    pub(super) plan: EntityId,
    pub(super) node: EntityId,
    pub(super) text: &'a str,
    pub(super) sources: &'a [super::super::super::PlanSemanticReference],
}
/// Converts or evaluates `SummaryLinkPayload`.

#[derive(Serialize)]
pub(super) struct SummaryLinkPayload {
    pub(super) schema_version: u16,
    pub(super) plan: EntityId,
    pub(super) node: EntityId,
    pub(super) summary: EntityId,
}
/// Converts or evaluates `GatePayload`.

#[derive(Serialize)]
pub(super) struct GatePayload<'a> {
    pub(super) schema_version: u16,
    pub(super) plan: EntityId,
    pub(super) owner: EntityId,
    pub(super) phase: super::super::super::PlanGatePhase,
    pub(super) requirement: &'a super::super::super::PlanGateRequirement,
    pub(super) waiver: &'a Option<super::super::super::PlanGateWaiver>,
}
/// Converts or evaluates `EvidencePayload`.

#[derive(Serialize)]
pub(super) struct EvidencePayload<'a> {
    pub(super) schema_version: u16,
    pub(super) plan: EntityId,
    pub(super) owner: super::super::super::PlanEvidenceOwner,
    pub(super) role: super::super::super::PlanEvidenceRole,
    pub(super) evidence: &'a super::super::super::PlanEvidenceReference,
    pub(super) note: &'a Option<String>,
    pub(super) actor: Option<EntityId>,
}
