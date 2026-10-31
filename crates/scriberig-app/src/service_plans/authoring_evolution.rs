//! Implements ordinary plan evolution mutations.

use crate::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationService,
    PlanDecomposeRequest, PlanNextStepRequest, SessionBackend,
};
use scriberig_session::{Plan, PlanNode, PlanNodeParent, PlanNodeStatus};

use super::validation::{invalid_plan, revision_conflict, validate_assignment};

impl<B: ApplicationBackend> ApplicationService<B> {
    /// Decomposes one plan node into structural child work.
    ///
    /// Required decompositions create typed completion gates. The operation
    /// does not add dependency edges between the new children.
    ///
    /// # Parameters
    /// - `self`: Application service owning plan persistence.
    /// - `context`: Authorized caller context.
    /// - `request`: Parent, exact plan revision, children, and completion policy.
    ///
    /// The authoritative updated plan with allocated child identities.
    ///
    /// Returns a revision conflict, invalid decomposition, assignment, lookup,
    /// or persistence error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn decompose_plan_node(
        &self,
        context: &ApplicationContext,
        request: &PlanDecomposeRequest,
    ) -> Result<Plan, ApplicationError> {
        let catalog_snapshot = self.model_catalog.snapshot();
        let mut children = Vec::with_capacity(request.children.len());
        for child in &request.children {
            if let Some(assignment) = &child.assignment {
                validate_assignment(&catalog_snapshot.models, assignment)?;
            }
            children.push(PlanNode {
                id: self.next_entity(0xC6),
                parent: Some(PlanNodeParent::Node(request.parent)),
                title: child.title.clone(),
                purpose: child.purpose.clone(),
                expected_result: child.expected_result.clone(),
                semantic_scope: child.semantic_scope.clone(),
                status: PlanNodeStatus::Planned,
                branch: None,
                result: None,
                superseded_by: None,
                abandon_reason: None,
                last_failure: None,
                assignment: child.assignment.clone(),
                claim: None,
                claim_generation: 0,
            });
        }
        let mut session = self.read_session(
            context,
            &crate::SessionRequest {
                session: request.session,
            },
        )?;
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
        plan.decompose(request.parent, children, request.required)
            .map_err(|error| invalid_plan(format!("invalid plan decomposition: {error:?}")))?;
        let updated = plan.clone();
        self.compare_and_swap_plan(context, &session, request.plan, request.expected_revision)?;
        self.operations.publish_plan_activity(
            request.session,
            request.plan,
            Some(request.parent),
            scriberig_session::SessionActivityKind::PlanRevised,
            scriberig_session::SessionActivityState::Completed,
            "Plan node decomposed",
        );
        Ok(updated)
    }

    /// Adds ordinary work to a running plan at its structural anchor.
    ///
    /// This operation preserves existing work and records one plan revision;
    /// replacement and dependency edits remain the responsibility of replan.
    ///
    /// # Parameters
    /// - `self`: Application service owning plan persistence.
    /// - `context`: Authorized caller context.
    /// - `request`: Exact plan revision and proposed next-step content.
    ///
    /// The authoritative updated plan with its allocated node identity.
    ///
    /// Returns a revision conflict, invalid work item, assignment, lookup, or
    /// persistence error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn add_plan_next_step(
        &self,
        context: &ApplicationContext,
        request: &PlanNextStepRequest,
    ) -> Result<Plan, ApplicationError> {
        let catalog_snapshot = self.model_catalog.snapshot();
        if let Some(assignment) = &request.assignment {
            validate_assignment(&catalog_snapshot.models, assignment)?;
        }
        let mut session = self.read_session(
            context,
            &crate::SessionRequest {
                session: request.session,
            },
        )?;
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
        let branch = plan
            .anchor
            .as_ref()
            .map(|anchor| match anchor {
                scriberig_session::ContinuationAnchor::BranchHead { branch }
                | scriberig_session::ContinuationAnchor::AssistantOutput { branch, .. }
                | scriberig_session::ContinuationAnchor::HumanInput { branch, .. }
                | scriberig_session::ContinuationAnchor::BranchResult { branch, .. } => *branch,
            })
            .ok_or_else(|| invalid_plan("plan has no structural anchor"))?;
        let node_id = self.next_entity(0xC5);
        let node = PlanNode {
            id: node_id,
            parent: Some(PlanNodeParent::Branch(branch)),
            title: request.title.clone(),
            purpose: request.purpose.clone(),
            expected_result: request.expected_result.clone(),
            semantic_scope: request.semantic_scope.clone(),
            status: PlanNodeStatus::Planned,
            branch: None,
            result: None,
            superseded_by: None,
            abandon_reason: None,
            last_failure: None,
            assignment: request.assignment.clone(),
            claim: None,
            claim_generation: 0,
        };
        plan.add_next_step(node)
            .map_err(|error| invalid_plan(format!("invalid next step: {error:?}")))?;
        let updated = plan.clone();
        self.compare_and_swap_plan(context, &session, request.plan, request.expected_revision)?;
        self.operations.publish_plan_activity(
            request.session,
            request.plan,
            Some(node_id),
            scriberig_session::SessionActivityKind::PlanRevised,
            scriberig_session::SessionActivityState::Completed,
            "Plan next step added",
        );
        Ok(updated)
    }
}
