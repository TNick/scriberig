//! Verifies legacy plan projections before semantic snapshot promotion.

use scriberig_session::{Plan, PlanSemanticSnapshot};

/// Rejects a semantic migration that changes the legacy plan projection.
///
/// Comparison includes compact plan/node views and the canonical dependency,
/// gate, evidence, and node records that carry result and topology semantics.
/// The target snapshot is intentionally absent because promotion has not
/// happened yet.
///
/// # Parameters
/// - `legacy`: Aggregate projection being migrated.
/// - `semantic`: Independently encoded semantic objects to promote.
///
/// # Returns
/// Unit when both representations have equivalent product semantics.
///
/// # Errors
/// Returns a diagnostic when decoding fails or any compared semantics differ.
pub(super) fn verify_before_promotion(
    legacy: &Plan,
    semantic: &PlanSemanticSnapshot,
) -> Result<(), String> {
    let decoded = semantic
        .clone()
        .into_plan(legacy.history.clone())
        .map_err(|error| format!("migration equivalence decode failed: {error:?}"))?;
    let legacy_view = legacy
        .plan_view(None)
        .map_err(|error| format!("legacy PlanView assembly failed: {error:?}"))?;
    let semantic_view = decoded
        .plan_view(None)
        .map_err(|error| format!("semantic PlanView assembly failed: {error:?}"))?;
    let legacy_rollups = legacy
        .node_rollups()
        .map_err(|error| format!("legacy node rollup failed: {error:?}"))?;
    let semantic_rollups = decoded
        .node_rollups()
        .map_err(|error| format!("semantic node rollup failed: {error:?}"))?;
    let legacy_nodes = legacy
        .nodes
        .iter()
        .map(|node| legacy.node_view(node.id, None, None, &legacy_rollups))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("legacy PlanNodeView assembly failed: {error:?}"))?;
    let semantic_nodes = decoded
        .nodes
        .iter()
        .map(|node| decoded.node_view(node.id, None, None, &semantic_rollups))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("semantic PlanNodeView assembly failed: {error:?}"))?;
    if legacy_view != semantic_view
        || legacy_nodes != semantic_nodes
        || legacy.nodes != decoded.nodes
        || legacy.dependencies != decoded.dependencies
        || legacy.gates != decoded.gates
        || legacy.evidence != decoded.evidence
    {
        return Err(format!(
            "semantic migration changes PlanView semantics for plan {} revision {}",
            legacy.id, legacy.revision
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use scriberig_core::EntityId;
    use scriberig_session::{
        ContinuationAnchor, Plan, PlanNode, PlanNodeParent, PlanNodeStatus, PlanSemanticSnapshot,
    };

    use super::verify_before_promotion;

    /// Builds a minimal plan with one stable node.
    ///
    /// # Returns
    /// A plan containing result-bearing completed work.
    fn fixture() -> Plan {
        let session = EntityId::from_bytes([1; 16]);
        let branch = EntityId::from_bytes([2; 16]);
        let mut plan = Plan::new(
            EntityId::from_bytes([3; 16]),
            session,
            ContinuationAnchor::BranchHead { branch },
            "migration gate",
        )
        .expect("construct plan");
        plan.add_node(PlanNode {
            id: EntityId::from_bytes([4; 16]),
            parent: Some(PlanNodeParent::Branch(branch)),
            title: "preserved node".to_owned(),
            purpose: "preserve status and result".to_owned(),
            expected_result: None,
            semantic_scope: Vec::new(),
            status: PlanNodeStatus::Completed,
            branch: Some(EntityId::from_bytes([5; 16])),
            result: Some(EntityId::from_bytes([6; 16])),
            superseded_by: None,
            abandon_reason: None,
            last_failure: None,
            assignment: None,
            claim: None,
            claim_generation: 0,
        })
        .expect("add node");
        plan
    }

    /// Accepts the exact semantic encoding of the legacy aggregate.
    #[test]
    fn accepts_equivalent_plan_before_promotion() {
        let plan = fixture();
        let semantic = PlanSemanticSnapshot::from_plan(&plan).expect("encode plan");

        verify_before_promotion(&plan, &semantic).expect("equivalent projection");
    }

    /// Rejects a semantic object set with altered node status before promotion.
    #[test]
    fn rejects_changed_node_status_before_promotion() {
        let plan = fixture();
        let mut changed = plan.clone();
        changed.nodes[0].status = PlanNodeStatus::Failed;
        let semantic = PlanSemanticSnapshot::from_plan(&changed).expect("encode changed plan");

        assert!(verify_before_promotion(&plan, &semantic).is_err());
    }
}
