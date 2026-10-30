//! Converts aggregate plan history to and from semantic revisions.

use scriberig_core::EncodingVersion;
use scriberig_session::{Plan, PlanRevision};
use scriberig_store::{Revision, SnapshotChange};

/// Projects the current semantic plan state into its history record shape.
///
/// # Parameters
/// - `plan`: Plan state to retain in the historical view.
///
/// The compact historical state for one mutation revision.
/// # Returns
/// A history record representing the supplied revision.
pub(super) fn plan_revision(plan: &Plan) -> PlanRevision {
    PlanRevision {
        revision: plan.revision,
        goal: plan.goal.clone(),
        status: plan.status,
        nodes: plan.nodes.clone(),
        dependencies: plan.dependencies.clone(),
        summaries: plan.summaries.clone(),
        evidence: plan.evidence.clone(),
        gates: plan.gates.clone(),
        reason: None,
    }
}

/// Reconstructs one legacy historical state using current invariant fields.
///
/// # Parameters
/// - `current`: Plan supplying stable owner, anchor, and coordinator fields.
/// - `revision`: Legacy graph state to restore.
///
/// The reconstructed plan state at `revision.revision`.
/// # Returns
/// The historical plan projection.
pub(super) fn plan_at_revision(current: &Plan, revision: &PlanRevision) -> Plan {
    let mut historical = current.clone();
    historical.revision = revision.revision;
    historical.goal.clone_from(&revision.goal);
    historical.status = revision.status;
    historical.nodes.clone_from(&revision.nodes);
    historical.dependencies.clone_from(&revision.dependencies);
    historical.summaries.clone_from(&revision.summaries);
    historical.evidence.clone_from(&revision.evidence);
    historical.gates.clone_from(&revision.gates);
    historical.root_nodes = historical
        .nodes
        .iter()
        .filter(|node| {
            matches!(
                node.parent,
                None | Some(scriberig_session::PlanNodeParent::Branch(_))
            )
        })
        .map(|node| node.id)
        .collect();
    historical
}

/// Converts semantic objects into universal revisions and snapshot changes.
///
/// # Parameters
/// - `objects`: Independently revisioned semantic plan objects.
///
/// # Returns
/// Universal revision records and their snapshot mappings.
///
/// # Errors
/// Returns an error when a semantic revision fails identity validation.
pub(super) fn semantic_writes(
    objects: Vec<scriberig_session::PlanSemanticRevision>,
) -> Result<(Vec<Revision>, Vec<SnapshotChange>), String> {
    let mut revisions = Vec::with_capacity(objects.len());
    let mut changes = Vec::with_capacity(objects.len());
    for object in objects {
        let revision = Revision::from_parts(
            object.revision,
            EncodingVersion::V1,
            object.object_type,
            object.payload,
        )
        .map_err(|error| format!("invalid plan revision: {error:?}"))?;
        changes.push(SnapshotChange::Set(object.entity, revision.id()));
        revisions.push(revision);
    }
    Ok((revisions, changes))
}
