//! Defines typed payloads used when writing semantic plan objects.

use scriberig_core::EntityId;
use serde::Serialize;

use super::super::super::{
    CoordinatorClaim, ExpectedResult, ModelAssignment, PlanNodeParent, PlanNodeStatus, PlanStatus,
    WorkerClaim,
};

/// Canonical serialized plan metadata payload.
///
/// # Public members
/// None.
///
/// # Private members
/// - `schema_version`: Payload format version.
/// - `session`: Owning session identity.
/// - `anchor`: Optional continuation anchor.
/// - `goal`: Plan goal text.
/// - `mutation_revision`: Monotonic plan revision number.
/// - `status`: Current plan lifecycle status.
/// - `root_branch`: Root branch identity.
/// - `coordinator`: Assigned coordinator identity.
/// - `coordinator_cursor`: Current coordinator cursor.
/// - `coordinator_claim`: Current coordinator claim.
/// - `coordinator_generation`: Coordinator claim generation.
#[derive(Serialize)]
pub(super) struct PlanPayload<'a> {
    pub(super) schema_version: u16,
    pub(super) session: Option<EntityId>,
    pub(super) anchor: Option<&'a crate::ContinuationAnchor>,
    pub(super) goal: &'a str,
    pub(super) mutation_revision: u64,
    pub(super) status: PlanStatus,
    pub(super) root_branch: EntityId,
    pub(super) coordinator: Option<EntityId>,
    pub(super) coordinator_cursor: Option<EntityId>,
    pub(super) coordinator_claim: Option<&'a CoordinatorClaim>,
    pub(super) coordinator_generation: u64,
}

/// Canonical serialized plan-node metadata payload.
///
/// # Public members
/// None.
///
/// # Private members
/// - `schema_version`: Payload format version.
/// - `plan`: Owning plan identity.
/// - `title`: Node title.
/// - `purpose`: Node purpose.
/// - `expected_result`: Optional expected result.
/// - `semantic_scope`: Domain-specific scope labels.
/// - `status`: Current node lifecycle status.
/// - `branch`: Assigned branch identity.
/// - `result`: Recorded branch result identity.
/// - `superseded_by`: Replacement node identity.
/// - `assignment`: Optional execution assignment.
/// - `claim`: Optional worker claim.
/// - `claim_generation`: Worker claim generation.
#[derive(Serialize)]
pub(super) struct NodePayload<'a> {
    pub(super) schema_version: u16,
    pub(super) plan: EntityId,
    pub(super) title: &'a str,
    pub(super) purpose: &'a str,
    pub(super) expected_result: &'a Option<ExpectedResult>,
    pub(super) semantic_scope: &'a [String],
    pub(super) status: PlanNodeStatus,
    pub(super) branch: Option<EntityId>,
    pub(super) result: Option<EntityId>,
    pub(super) superseded_by: Option<EntityId>,
    pub(super) assignment: &'a Option<ModelAssignment>,
    pub(super) claim: &'a Option<WorkerClaim>,
    pub(super) claim_generation: u64,
}

/// Canonical serialized relation that orders a node within its plan.
///
/// # Public members
/// None.
///
/// # Private members
/// - `schema_version`: Payload format version.
/// - `plan`: Owning plan identity.
/// - `node`: Ordered node identity.
/// - `ordinal`: Stable node order.
#[derive(Serialize)]
pub(super) struct MembershipPayload {
    pub(super) schema_version: u16,
    pub(super) plan: EntityId,
    pub(super) node: EntityId,
    pub(super) ordinal: u64,
}

/// Canonical serialized structural parent relation for a plan node.
///
/// # Public members
/// None.
///
/// # Private members
/// - `schema_version`: Payload format version.
/// - `plan`: Owning plan identity.
/// - `root_branch`: Root branch identity.
/// - `child`: Structurally nested node.
/// - `parent`: Optional parent node or branch.
/// - `ordinal`: Stable sibling order.
#[derive(Serialize)]
pub(super) struct StructurePayload {
    pub(super) schema_version: u16,
    pub(super) plan: EntityId,
    pub(super) root_branch: EntityId,
    pub(super) child: EntityId,
    pub(super) parent: Option<PlanNodeParent>,
    pub(super) ordinal: u64,
}

/// Canonical serialized prerequisite relation between plan nodes.
///
/// # Public members
/// None.
///
/// # Private members
/// - `schema_version`: Payload format version.
/// - `plan`: Owning plan identity.
/// - `prerequisite`: Required node identity.
/// - `dependent`: Blocked node identity.
/// - `fulfilled_by`: Optional evidence identity satisfying the edge.
#[derive(Serialize)]
pub(super) struct DependencyPayload {
    pub(super) schema_version: u16,
    pub(super) plan: EntityId,
    pub(super) prerequisite: EntityId,
    pub(super) dependent: EntityId,
    pub(super) fulfilled_by: Option<EntityId>,
}

/// Stable identity fields for a semantic relation.
///
/// # Public members
/// - `owner`: Relation owner identity.
/// - `target`: Related entity identity.
///
/// # Private members
/// None.
#[derive(Serialize)]
pub(in crate::planning::semantic) struct RelationIdentity {
    pub(in crate::planning::semantic) owner: EntityId,
    pub(in crate::planning::semantic) target: EntityId,
}

/// Stable identity fields for a dependency relation.
///
/// # Public members
/// - `plan`: Owning plan identity.
/// - `prerequisite`: Required node identity.
/// - `dependent`: Blocked node identity.
///
/// # Private members
/// None.
#[derive(Serialize)]
pub(in crate::planning::semantic) struct DependencyIdentity {
    pub(in crate::planning::semantic) plan: EntityId,
    pub(in crate::planning::semantic) prerequisite: EntityId,
    pub(in crate::planning::semantic) dependent: EntityId,
}
