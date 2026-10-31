//! Applies typed plan mutations through one optimistic application boundary.

use crate::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationService, PlanMutation,
    PlanMutationRequest, PlanNodeDraft, SessionBackend, SessionRequest,
};
use scriberig_core::EntityId;
use scriberig_session::{
    Plan, PlanEvidenceAssociation, PlanGate, PlanGateWaiver, PlanNode, PlanNodeParent,
    PlanNodeStatus, PlanNodeSummary,
};

use super::validation::{
    invalid_plan, revision_conflict, validate_assignment, validate_replan_drafts,
};

impl<B: ApplicationBackend> ApplicationService<B> {
    /// Applies one typed semantic mutation against an observed plan revision.
    ///
    /// - `self`: Application service owning plan persistence.
    /// - `context`: Authorized caller context.
    /// - `request`: Plan identity, observed revision, and typed mutation.
    ///
    /// The authoritative plan after one atomic mutation.
    ///
    /// Returns authorization, validation, stale revision, or persistence errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the mutation is invalid or cannot be persisted.
    pub fn apply_plan_mutation(
        &self,
        context: &ApplicationContext,
        request: &PlanMutationRequest,
    ) -> Result<Plan, ApplicationError> {
        let mut session = self.read_session(
            context,
            &SessionRequest {
                session: request.session,
            },
        )?;
        let results = session
            .branches
            .iter()
            .filter_map(|branch| branch.result.clone())
            .collect::<Vec<_>>();
        let plan = session
            .plans
            .iter_mut()
            .find(|plan| plan.id == request.plan)
            .ok_or_else(|| ApplicationError::NotFound {
                subject: request.plan.to_string(),
            })?;
        if plan.revision != request.expected_revision {
            return Err(revision_conflict(request.plan));
        }
        let catalog = self.provider_extension_runtime.model_catalog().snapshot();
        apply_mutation(
            self,
            plan,
            request.mutation.clone(),
            &results,
            &catalog.models,
            context
                .principal
                .as_ref()
                .and_then(|principal| principal.id.parse().ok()),
        )?;
        let updated = plan.clone();
        self.compare_and_swap_plan(context, &session, request.plan, request.expected_revision)?;
        self.operations.publish_plan_activity(
            request.session,
            request.plan,
            mutation_node(&request.mutation),
            scriberig_session::SessionActivityKind::PlanRevised,
            scriberig_session::SessionActivityState::Completed,
            "Plan updated",
        );
        Ok(updated)
    }
}

/// Applies one domain mutation while preserving the aggregate's atomic rules.
///
/// - `service`: Allocates stable identities for new semantic records.
/// - `plan`: Mutable plan validated by the caller's expected revision.
/// - `mutation`: Typed operation to apply.
/// - `results`: Exact branch results available for retry validation.
/// - `models`: Upstream model catalog used to validate assignments.
/// - `actor`: Trusted semantic identity of the caller, when available.
///
/// Unit after the domain mutation advances the plan-local revision once.
///
/// Returns validation and domain-transition errors without partial mutation.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the proposed mutation is invalid.
fn apply_mutation<B: ApplicationBackend>(
    service: &ApplicationService<B>,
    plan: &mut Plan,
    mutation: PlanMutation,
    results: &[scriberig_session::BranchResult],
    models: &[rpi_ai::Model],
    actor: Option<EntityId>,
) -> Result<(), ApplicationError> {
    match mutation {
        PlanMutation::AddNextStep(draft) => {
            validate_drafts(std::slice::from_ref(&draft), models)?;
            let branch = anchor_branch(plan)?;
            let node = node_from_draft(service, draft, Some(PlanNodeParent::Branch(branch)));
            plan.add_next_step(node)
                .map_err(|error| invalid_plan(format!("next step rejected: {error:?}")))
        }
        PlanMutation::Decompose {
            parent,
            children,
            required,
        } => {
            validate_drafts(&children, models)?;
            if children.iter().any(|child| child.parent_index.is_some()) {
                return Err(invalid_plan(
                    "decomposition children must share the selected parent",
                ));
            }
            let nodes = children
                .into_iter()
                .map(|draft| node_from_draft(service, draft, Some(PlanNodeParent::Node(parent))))
                .collect();
            plan.decompose(parent, nodes, required)
                .map_err(|error| invalid_plan(format!("decomposition rejected: {error:?}")))
        }
        PlanMutation::AddGate {
            owner,
            phase,
            requirement,
        } => plan
            .add_gate(PlanGate {
                id: service.next_entity(0xC5),
                owner,
                phase,
                requirement,
                waiver: None,
            })
            .map_err(|error| invalid_plan(format!("gate rejected: {error:?}"))),
        PlanMutation::WaiveGate { gate, reason } => plan
            .waive_gate(
                gate,
                PlanGateWaiver {
                    reason,
                    actor,
                    operation: None,
                },
            )
            .map_err(|error| invalid_plan(format!("gate waiver rejected: {error:?}"))),
        PlanMutation::AttachEvidence {
            owner,
            role,
            evidence,
            note,
        } => plan
            .attach_evidence(PlanEvidenceAssociation {
                id: service.next_entity(0xC6),
                owner,
                role,
                evidence,
                note,
                actor,
            })
            .map_err(|error| invalid_plan(format!("evidence rejected: {error:?}"))),
        PlanMutation::UpdateSummary {
            node,
            text,
            sources,
        } => plan
            .update_summary(PlanNodeSummary {
                id: service.next_entity(0xC7),
                node,
                text,
                sources,
            })
            .map_err(|error| invalid_plan(format!("summary rejected: {error:?}"))),
        PlanMutation::RetryFailedNode { node } => plan
            .retry_failed_node(node, results)
            .map_err(|error| invalid_plan(format!("node retry rejected: {error:?}"))),
        PlanMutation::RecoverFailedPlan => plan
            .recover_failed_plan()
            .map_err(|error| invalid_plan(format!("plan recovery rejected: {error:?}"))),
    }
}

/// Validates proposed node text and model assignments before allocating nodes.
///
/// - `drafts`: New work items with request-local keys.
/// - `models`: Current upstream model catalog.
///
/// Unit after every draft and optional assignment is valid.
///
/// Returns invalid-request for malformed drafts or stale model selections.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when a draft is invalid.
fn validate_drafts(
    drafts: &[PlanNodeDraft],
    models: &[rpi_ai::Model],
) -> Result<(), ApplicationError> {
    validate_replan_drafts(drafts)?;
    for draft in drafts {
        if let Some(assignment) = &draft.assignment {
            validate_assignment(models, assignment)?;
        }
    }
    Ok(())
}

/// Builds a new plan node with an application allocated semantic identity.
///
/// - `service`: Application identity allocator.
/// - `draft`: Validated provider-neutral node proposal.
/// - `parent`: Structural owner selected by the operation.
///
/// A new unstarted plan node.
fn node_from_draft<B>(
    service: &ApplicationService<B>,
    draft: PlanNodeDraft,
    parent: Option<PlanNodeParent>,
) -> PlanNode {
    PlanNode {
        id: service.next_entity(0xC8),
        parent,
        title: draft.title,
        purpose: draft.purpose,
        expected_result: draft.expected_result,
        semantic_scope: draft.semantic_scope,
        status: PlanNodeStatus::Planned,
        branch: None,
        result: None,
        superseded_by: None,
        abandon_reason: None,
        last_failure: None,
        assignment: draft.assignment,
        claim: None,
        claim_generation: 0,
    }
}

/// Resolves the branch that anchors an ordinary next step.
///
/// - `plan`: Running plan with its exact continuation anchor.
///
/// # Returns
/// The branch identity used as the new node's structural parent.
///
/// # Errors
/// Returns an error when the plan has no supported branch anchor.
fn anchor_branch(plan: &Plan) -> Result<EntityId, ApplicationError> {
    plan.anchor
        .as_ref()
        .map(|anchor| match anchor {
            scriberig_session::ContinuationAnchor::BranchHead { branch }
            | scriberig_session::ContinuationAnchor::AssistantOutput { branch, .. }
            | scriberig_session::ContinuationAnchor::HumanInput { branch, .. }
            | scriberig_session::ContinuationAnchor::BranchResult { branch, .. } => *branch,
        })
        .ok_or_else(|| invalid_plan("plan has no structural anchor"))
}

/// Identifies the node affected by a mutation for activity projection.
///
/// - `mutation`: Typed operation that was applied.
///
/// # Returns
/// The affected node identity when the request names one.
fn mutation_node(mutation: &PlanMutation) -> Option<EntityId> {
    match mutation {
        PlanMutation::Decompose { parent, .. } => Some(*parent),
        PlanMutation::AddGate { owner, .. } => Some(*owner),
        PlanMutation::AttachEvidence { owner, .. } => match owner {
            scriberig_session::PlanEvidenceOwner::Node(node) => Some(*node),
            scriberig_session::PlanEvidenceOwner::Gate(_) => None,
        },
        PlanMutation::UpdateSummary { node, .. } | PlanMutation::RetryFailedNode { node } => {
            Some(*node)
        }
        PlanMutation::AddNextStep(_)
        | PlanMutation::WaiveGate { .. }
        | PlanMutation::RecoverFailedPlan => None,
    }
}
