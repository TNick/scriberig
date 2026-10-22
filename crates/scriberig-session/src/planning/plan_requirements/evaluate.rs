//! Evaluates typed plan gates from exact associated evidence.

use scriberig_core::EntityId;

use super::super::{
    Plan, PlanEvidenceKind, PlanEvidenceOwner, PlanEvidenceReference, PlanEvidenceRole,
    PlanGateEvaluation, PlanGatePhase, PlanGateRequirement, PlanGateStatus, PlanNodeStatus,
    PlanSemanticPredicate, PlanSemanticReference,
};

impl Plan {
    /// Evaluates one canonical gate using its exact evidence associations.
    ///
    /// # Parameters
    /// - `self`: Plan snapshot containing gate definitions and evidence.
    /// - `gate`: Stable gate identity to evaluate.
    ///
    /// The derived gate evaluation, or `None` for an unknown gate.
    #[must_use]
    pub fn evaluate_gate(&self, gate: EntityId) -> Option<PlanGateEvaluation> {
        let gate_record = self.gates.iter().find(|candidate| candidate.id == gate)?;
        if gate_record.waiver.is_some() {
            return Some(PlanGateEvaluation {
                gate,
                status: PlanGateStatus::Waived,
                supporting_evidence: Vec::new(),
                reason: gate_record.waiver.as_ref().map(|waiver| waiver.reason.clone()),
            });
        }
        if let PlanGateRequirement::RequiredChildCompleted { child } = &gate_record.requirement {
            let completed = self
                .nodes
                .iter()
                .any(|node| node.id == *child && node.status == PlanNodeStatus::Completed);
            return Some(PlanGateEvaluation {
                gate,
                status: if completed {
                    PlanGateStatus::Satisfied
                } else if self
                    .nodes
                    .iter()
                    .any(|node| node.id == *child && node.status == PlanNodeStatus::Failed)
                {
                    PlanGateStatus::Failed
                } else {
                    PlanGateStatus::Pending
                },
                supporting_evidence: Vec::new(),
                reason: (!completed).then(|| "required child is not complete".to_owned()),
            });
        }
        let associations = self
            .evidence
            .iter()
            .filter(|association| association.owner == PlanEvidenceOwner::Gate(gate))
            .collect::<Vec<_>>();
        let supporting = associations
            .iter()
            .filter(|association| {
                association.role == PlanEvidenceRole::Supports
                    && satisfies_requirement(&gate_record.requirement, &association.evidence)
            })
            .map(|association| association.evidence.clone())
            .collect::<Vec<_>>();
        let contradicting = associations.iter().any(|association| {
            association.role == PlanEvidenceRole::Contradicts
                && relevant_to_requirement(&gate_record.requirement, &association.evidence)
        }) || associations.iter().any(|association| {
            association.role == PlanEvidenceRole::Supports
                && failed_requirement(&gate_record.requirement, &association.evidence)
        });
        let status = if contradicting {
            PlanGateStatus::Failed
        } else if !supporting.is_empty() {
            PlanGateStatus::Satisfied
        } else {
            PlanGateStatus::Pending
        };
        Some(PlanGateEvaluation {
            gate,
            status,
            supporting_evidence: supporting,
            reason: match status {
                PlanGateStatus::Pending => Some("required evidence is pending".to_owned()),
                PlanGateStatus::Failed => Some("evidence contradicts the requirement".to_owned()),
                PlanGateStatus::Satisfied | PlanGateStatus::Waived => None,
            },
        })
    }

    /// Evaluates every gate attached to a node for the requested lifecycle phase.
    ///
    /// # Parameters
    /// - `self`: Plan snapshot containing gate definitions and evidence.
    /// - `node`: Stable plan node identity.
    /// - `phase`: Entry or completion phase to inspect.
    ///
    /// Ordered evaluations for matching gates.
    #[must_use]
    pub fn evaluate_node_gates(
        &self,
        node: EntityId,
        phase: PlanGatePhase,
    ) -> Vec<PlanGateEvaluation> {
        let mut gates = self
            .gates
            .iter()
            .filter(|gate| gate.owner == node && gate.phase == phase)
            .filter_map(|gate| self.evaluate_gate(gate.id))
            .collect::<Vec<_>>();
        gates.sort_by_key(|evaluation| evaluation.gate);
        gates
    }

    /// Checks whether all gates of one lifecycle phase are satisfied or waived.
    ///
    /// # Parameters
    /// - `self`: Plan snapshot containing gate definitions and evidence.
    /// - `node`: Stable plan node identity.
    /// - `phase`: Entry or completion phase to inspect.
    ///
    /// `true` when no matching gate is pending or failed.
    #[must_use]
    pub fn node_gates_satisfied(&self, node: EntityId, phase: PlanGatePhase) -> bool {
        self.evaluate_node_gates(node, phase).iter().all(|evaluation| {
            matches!(
                evaluation.status,
                PlanGateStatus::Satisfied | PlanGateStatus::Waived
            )
        })
    }
}

/// Determines whether one evidence reference supports a typed requirement.
///
/// # Parameters
/// - `requirement`: Typed canonical gate requirement.
/// - `evidence`: Exact evidence object being evaluated.
///
/// `true` when the exact evidence satisfies the requirement.
fn satisfies_requirement(
    requirement: &PlanGateRequirement,
    evidence: &PlanEvidenceReference,
) -> bool {
    match requirement {
        PlanGateRequirement::EvidencePresent { kind } => &evidence.kind == kind,
        PlanGateRequirement::ValidationPassed { profile } => {
            evidence.kind == PlanEvidenceKind::ValidationResult
                && evidence.attributes.get("passed").map(String::as_str) == Some("true")
                && profile.as_ref().is_none_or(|expected| {
                    evidence.attributes.get("profile") == Some(expected)
                })
        }
        PlanGateRequirement::DecisionResolved { decision } => {
            evidence.kind == PlanEvidenceKind::Decision
                && evidence.reference.entity == *decision
                && evidence.attributes.get("resolved").map(String::as_str) == Some("true")
        }
        PlanGateRequirement::HumanApproval { policy, role } => {
            evidence.kind == PlanEvidenceKind::HumanApproval
                && evidence.attributes.get("policy") == Some(policy)
                && role.as_ref().is_none_or(|expected| {
                    evidence.attributes.get("role") == Some(expected)
                })
        }
        PlanGateRequirement::SemanticCondition {
            reference,
            predicate,
        } => semantic_reference_matches(reference, &evidence.reference)
            && match predicate {
                PlanSemanticPredicate::EntityExists => true,
                PlanSemanticPredicate::ExactRevision => reference
                    .revision
                    .is_some_and(|revision| evidence.reference.revision == Some(revision)),
            },
        PlanGateRequirement::RequiredChildCompleted { .. } => false,
    }
}

/// Determines whether evidence pertains to a requirement even if it failed.
///
/// # Parameters
/// - `requirement`: Typed canonical gate requirement.
/// - `evidence`: Exact evidence object being evaluated.
///
/// `true` when the evidence is a relevant failed/contradicting result.
fn relevant_to_requirement(
    requirement: &PlanGateRequirement,
    evidence: &PlanEvidenceReference,
) -> bool {
    match requirement {
        PlanGateRequirement::EvidencePresent { kind } => &evidence.kind == kind,
        PlanGateRequirement::ValidationPassed { profile } => {
            evidence.kind == PlanEvidenceKind::ValidationResult
                && profile.as_ref().is_none_or(|expected| {
                    evidence.attributes.get("profile") == Some(expected)
                })
        }
        PlanGateRequirement::DecisionResolved { decision } => {
            evidence.kind == PlanEvidenceKind::Decision
                && evidence.reference.entity == *decision
        }
        PlanGateRequirement::HumanApproval { policy, role } => {
            evidence.kind == PlanEvidenceKind::HumanApproval
                && evidence.attributes.get("policy") == Some(policy)
                && role.as_ref().is_none_or(|expected| {
                    evidence.attributes.get("role") == Some(expected)
                })
        }
        PlanGateRequirement::SemanticCondition { reference, .. } => {
            semantic_reference_matches(reference, &evidence.reference)
        }
        PlanGateRequirement::RequiredChildCompleted { .. } => false,
    }
}

/// Identifies typed evidence that explicitly records a failed requirement.
///
/// # Parameters
/// - `requirement`: Typed canonical gate requirement.
/// - `evidence`: Exact evidence object being evaluated.
///
/// `true` when a relevant source carries an explicit negative result.
fn failed_requirement(
    requirement: &PlanGateRequirement,
    evidence: &PlanEvidenceReference,
) -> bool {
    relevant_to_requirement(requirement, evidence)
        && match requirement {
            PlanGateRequirement::ValidationPassed { .. } => {
                evidence.attributes.get("passed").map(String::as_str) == Some("false")
            }
            PlanGateRequirement::DecisionResolved { .. } => {
                evidence.attributes.get("resolved").map(String::as_str) == Some("false")
            }
            _ => false,
        }
}

/// Checks exact semantic reference equality without weakening revision bounds.
///
/// # Parameters
/// - `expected`: Exact reference required by the gate.
/// - `actual`: Exact reference carried by the evidence.
///
/// `true` when entity, revision, and snapshot constraints match.
fn semantic_reference_matches(
    expected: &PlanSemanticReference,
    actual: &PlanSemanticReference,
) -> bool {
    expected.entity == actual.entity
        && expected.revision.is_none_or(|revision| actual.revision == Some(revision))
        && expected.snapshot.is_none_or(|snapshot| actual.snapshot == Some(snapshot))
}

