//! Defines typed payloads used by the semantic plan decoder.

use super::super::super::{
    CoordinatorClaim, ExpectedResult, ModelAssignment, PlanEvidenceOwner, PlanEvidenceReference,
    PlanEvidenceRole, PlanGatePhase, PlanGateRequirement, PlanGateWaiver, PlanNodeParent,
    PlanNodeStatus, PlanStatus, WorkerClaim,
};
use scriberig_core::EntityId;
use serde::Deserialize;

/// Represents the immutable plan projection payload.
#[derive(Deserialize)]
pub(super) struct PlanPayload {
    pub(super) schema_version: u16,
    pub(super) session: Option<EntityId>,
    pub(super) anchor: Option<crate::ContinuationAnchor>,
    pub(super) goal: String,
    pub(super) mutation_revision: u64,
    pub(super) status: PlanStatus,
    pub(super) root_branch: EntityId,
    pub(super) coordinator: Option<EntityId>,
    pub(super) coordinator_cursor: Option<EntityId>,
    pub(super) coordinator_claim: Option<CoordinatorClaim>,
    pub(super) coordinator_generation: u64,
}
/// Converts or evaluates `NodePayload`.

#[derive(Deserialize)]
pub(super) struct NodePayload {
    pub(super) schema_version: u16,
    pub(super) plan: EntityId,
    pub(super) title: String,
    pub(super) purpose: String,
    pub(super) expected_result: Option<ExpectedResult>,
    pub(super) semantic_scope: Vec<String>,
    pub(super) status: PlanNodeStatus,
    pub(super) branch: Option<EntityId>,
    pub(super) result: Option<EntityId>,
    pub(super) superseded_by: Option<EntityId>,
    #[serde(default)]
    pub(super) abandon_reason: Option<super::super::super::PlanAbandonReason>,
    #[serde(default)]
    pub(super) last_failure: Option<PlanEvidenceReference>,
    pub(super) assignment: Option<ModelAssignment>,
    pub(super) claim: Option<WorkerClaim>,
    pub(super) claim_generation: u64,
}
/// Converts or evaluates `MembershipPayload`.

#[derive(Deserialize)]
pub(super) struct MembershipPayload {
    pub(super) schema_version: u16,
    pub(super) plan: EntityId,
    pub(super) node: EntityId,
    pub(super) ordinal: u64,
}
/// Converts or evaluates `StructurePayload`.

#[derive(Deserialize)]
pub(super) struct StructurePayload {
    pub(super) schema_version: u16,
    pub(super) plan: EntityId,
    pub(super) root_branch: EntityId,
    pub(super) child: EntityId,
    pub(super) parent: Option<PlanNodeParent>,
    pub(super) ordinal: u64,
    #[serde(default)]
    pub(super) root_ordinal: Option<u64>,
}
/// Converts or evaluates `DependencyPayload`.

#[derive(Deserialize)]
pub(super) struct DependencyPayload {
    pub(super) schema_version: u16,
    pub(super) plan: EntityId,
    pub(super) prerequisite: EntityId,
    pub(super) dependent: EntityId,
    pub(super) fulfilled_by: Option<EntityId>,
}
/// Converts or evaluates `HistoryPayload`.

#[derive(Deserialize)]
pub(super) struct HistoryPayload {
    pub(super) schema_version: u16,
    pub(super) plan: EntityId,
    pub(super) mutation_revision: u64,
    pub(super) reason: Option<String>,
}
/// Converts or evaluates `HistoryIdentity`.

#[derive(serde::Serialize)]
pub(super) struct HistoryIdentity {
    pub(super) plan: EntityId,
    pub(super) mutation_revision: u64,
}
/// Converts or evaluates `SummaryPayload`.

#[derive(Deserialize)]
pub(super) struct SummaryPayload {
    pub(super) schema_version: u16,
    pub(super) plan: EntityId,
    pub(super) node: EntityId,
    pub(super) text: String,
    pub(super) sources: Vec<super::super::super::PlanSemanticReference>,
}
/// Converts or evaluates `SummaryLinkPayload`.

#[derive(Deserialize)]
pub(super) struct SummaryLinkPayload {
    pub(super) schema_version: u16,
    pub(super) plan: EntityId,
    pub(super) node: EntityId,
    pub(super) summary: EntityId,
}
/// Converts or evaluates `GatePayload`.

#[derive(Deserialize)]
pub(super) struct GatePayload {
    pub(super) schema_version: u16,
    pub(super) plan: EntityId,
    pub(super) owner: EntityId,
    pub(super) phase: PlanGatePhase,
    pub(super) requirement: PlanGateRequirement,
    pub(super) waiver: Option<PlanGateWaiver>,
}
/// Converts or evaluates `EvidencePayload`.

#[derive(Deserialize)]
pub(super) struct EvidencePayload {
    pub(super) schema_version: u16,
    pub(super) plan: EntityId,
    pub(super) owner: PlanEvidenceOwner,
    pub(super) role: PlanEvidenceRole,
    pub(super) evidence: PlanEvidenceReference,
    pub(super) note: Option<String>,
    pub(super) actor: Option<EntityId>,
}
