//! Encodes attempts, dependencies, summaries, gates, evidence, and history.

use super::super::{PlanSemanticError, PlanSemanticRevision};
use super::{
    ATTEMPT_TYPE, AttemptIdentity, AttemptPayload, DEPENDENCY_TYPE, DependencyIdentity,
    DependencyPayload, EVIDENCE_TYPE, EvidencePayload, GATE_TYPE, GatePayload, HISTORY_TYPE,
    HistoryIdentity, HistoryPayload, Plan, RelationIdentity, SUMMARY_LINK_TYPE, SUMMARY_TYPE,
    SummaryLinkPayload, SummaryPayload, push_object, push_relation, relation_id,
};

/// Adds non-structural semantic records for one plan.
///
/// # Parameters
/// - `plan`: Aggregate plan supplying current semantic details.
/// - `objects`: Object list receiving encoded records.
///
/// # Returns
/// Unit after every detail record has been encoded.
///
/// # Errors
/// Returns an error when a record cannot be serialized or identified.
pub(super) fn append_related(
    plan: &Plan,
    objects: &mut Vec<PlanSemanticRevision>,
) -> Result<(), PlanSemanticError> {
    append_attempts(plan, objects)?;
    append_dependencies_and_summaries(plan, objects)?;
    append_gates_evidence_and_history(plan, objects)
}

/// Encodes exact attempt history for every current plan node.
///
/// # Parameters
/// - `plan`: Aggregate plan supplying attempt provenance.
/// - `objects`: Object list receiving encoded attempts.
///
/// # Returns
/// Unit after all attempt records have been encoded.
///
/// # Errors
/// Returns an error when an attempt identity or payload cannot be encoded.
fn append_attempts(
    plan: &Plan,
    objects: &mut Vec<PlanSemanticRevision>,
) -> Result<(), PlanSemanticError> {
    for node in &plan.nodes {
        if let Some(attempts) = plan.attempt_history(node.id) {
            for attempt in attempts {
                let identity = AttemptIdentity {
                    plan: plan.id,
                    node: node.id,
                    branch: attempt.branch,
                    claim_generation: attempt.claim_generation,
                };
                let entity = relation_id(ATTEMPT_TYPE, &identity)?;
                push_object(
                    objects,
                    entity,
                    ATTEMPT_TYPE,
                    &AttemptPayload {
                        schema_version: 1,
                        plan: plan.id,
                        node: node.id,
                        branch: attempt.branch,
                        claim_generation: attempt.claim_generation,
                        claim: attempt.claim,
                        status: attempt.status,
                        result: attempt.result,
                        failure: &attempt.failure,
                    },
                )?;
            }
        }
    }
    Ok(())
}

/// Encodes dependency edges and node summary records.
///
/// # Parameters
/// - `plan`: Aggregate plan supplying dependency and summary state.
/// - `objects`: Object list receiving encoded records.
///
/// # Returns
/// Unit after dependency and summary records are encoded.
///
/// # Errors
/// Returns an error when a record cannot be serialized or identified.
fn append_dependencies_and_summaries(
    plan: &Plan,
    objects: &mut Vec<PlanSemanticRevision>,
) -> Result<(), PlanSemanticError> {
    for dependency in &plan.dependencies {
        let payload = DependencyPayload {
            schema_version: 1,
            plan: plan.id,
            prerequisite: dependency.prerequisite,
            dependent: dependency.dependent,
            fulfilled_by: dependency.fulfilled_by,
        };
        push_relation(
            objects,
            DEPENDENCY_TYPE,
            &DependencyIdentity {
                plan: plan.id,
                prerequisite: dependency.prerequisite,
                dependent: dependency.dependent,
            },
            &payload,
        )?;
    }

    for summary in &plan.summaries {
        push_object(
            objects,
            summary.id,
            SUMMARY_TYPE,
            &SummaryPayload {
                schema_version: 1,
                plan: plan.id,
                node: summary.node,
                text: &summary.text,
                sources: &summary.sources,
            },
        )?;
        push_relation(
            objects,
            SUMMARY_LINK_TYPE,
            &RelationIdentity {
                owner: summary.node,
                target: summary.id,
            },
            &SummaryLinkPayload {
                schema_version: 1,
                plan: plan.id,
                node: summary.node,
                summary: summary.id,
            },
        )?;
    }
    Ok(())
}

/// Encodes typed gates, evidence associations, and mutation history.
///
/// # Parameters
/// - `plan`: Aggregate plan supplying requirements and provenance.
/// - `objects`: Object list receiving encoded records.
///
/// # Returns
/// Unit after gate, evidence, and history records are encoded.
///
/// # Errors
/// Returns an error when a record cannot be serialized or identified.
fn append_gates_evidence_and_history(
    plan: &Plan,
    objects: &mut Vec<PlanSemanticRevision>,
) -> Result<(), PlanSemanticError> {
    for gate in &plan.gates {
        push_object(
            objects,
            gate.id,
            GATE_TYPE,
            &GatePayload {
                schema_version: 1,
                plan: plan.id,
                owner: gate.owner,
                phase: gate.phase,
                requirement: &gate.requirement,
                waiver: &gate.waiver,
            },
        )?;
    }

    for association in &plan.evidence {
        push_object(
            objects,
            association.id,
            EVIDENCE_TYPE,
            &EvidencePayload {
                schema_version: 1,
                plan: plan.id,
                owner: association.owner,
                role: association.role,
                evidence: &association.evidence,
                note: &association.note,
                actor: association.actor,
            },
        )?;
    }

    for history in &plan.history {
        push_relation(
            objects,
            HISTORY_TYPE,
            &HistoryIdentity {
                plan: plan.id,
                mutation_revision: history.revision,
            },
            &HistoryPayload {
                schema_version: 1,
                plan: plan.id,
                mutation_revision: history.revision,
                reason: &history.reason,
            },
        )?;
    }

    Ok(())
}
