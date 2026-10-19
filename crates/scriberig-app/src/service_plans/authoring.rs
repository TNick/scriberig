//! Implements validated durable plan application operations.
use crate::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationService,
    PlanCreateRequest, PlanNodeReference, PlanReplanRequest, SessionBackend,
};
use scriberig_session::{Plan, PlanNode, PlanNodeParent, PlanNodeStatus, PlanStatus};

use super::validation::{
    invalid_plan, revision_conflict, validate_assignment, validate_proposal, validate_replan_drafts,
};

impl<B: ApplicationBackend> ApplicationService<B> {
    /// Applies a validated replan using one observed durable plan revision.
    ///
    /// - `self`: Application service owning plan persistence.
    /// - `context`: Authorized caller context.
    /// - `request`: New nodes, dependency changes, and explicit replacements.
    ///
    /// The latest plan projection with preserved completed results.
    ///
    /// Returns validation, revision conflict, assignment, or persistence errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    #[allow(clippy::too_many_lines)]
    pub fn replan(
        &self,
        context: &ApplicationContext,
        request: &PlanReplanRequest,
    ) -> Result<Plan, ApplicationError> {
        validate_replan_drafts(&request.additions)?;
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
        let node_ids = request
            .additions
            .iter()
            .map(|_| self.next_entity(0xC4))
            .collect::<Vec<_>>();
        let keys = request
            .additions
            .iter()
            .enumerate()
            .map(|(index, draft)| (draft.key.as_str(), node_ids[index]))
            .collect::<std::collections::BTreeMap<_, _>>();
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
        let mut additions = Vec::with_capacity(request.additions.len());
        for (index, draft) in request.additions.iter().enumerate() {
            if let Some(assignment) = &draft.assignment {
                validate_assignment(&self.model_library, assignment)?;
            }
            additions.push(PlanNode {
                id: node_ids[index],
                parent: Some(
                    draft
                        .parent_index
                        .map_or(PlanNodeParent::Branch(branch), |parent| {
                            PlanNodeParent::Node(node_ids[parent])
                        }),
                ),
                title: draft.title.clone(),
                purpose: draft.purpose.clone(),
                expected_result: draft.expected_result.clone(),
                semantic_scope: draft.semantic_scope.clone(),
                status: PlanNodeStatus::Planned,
                branch: None,
                result: None,
                superseded_by: None,
                assignment: draft.assignment.clone(),
                claim: None,
                claim_generation: 0,
            });
        }
        let resolve = |reference: &PlanNodeReference| match reference {
            PlanNodeReference::Existing(id) => Some(*id),
            PlanNodeReference::Proposed(key) => keys.get(key.as_str()).copied(),
        };
        let dependencies = request
            .dependencies
            .iter()
            .map(|(prerequisite, dependent)| {
                resolve(prerequisite)
                    .zip(resolve(dependent))
                    .ok_or_else(|| invalid_plan("dependency references an unknown node"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let superseded = request
            .superseded
            .iter()
            .map(|(old, key)| {
                keys.get(key.as_str())
                    .copied()
                    .map(|replacement| (*old, replacement))
                    .ok_or_else(|| invalid_plan("replacement references an unknown node"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        for (_, assignment) in &request.assignment_changes {
            if let Some(assignment) = assignment {
                validate_assignment(&self.model_library, assignment)?;
            }
        }
        plan.replan(
            additions,
            dependencies,
            &request.removed_dependencies,
            &superseded,
            &request.assignment_changes,
            &request.preserved_terminal_nodes,
            &request.reason,
        )
        .map_err(|error| invalid_plan(format!("invalid replan: {error:?}")))?;
        let updated = plan.clone();
        self.compare_and_swap_plan(context, &session, request.plan, request.expected_revision)?;
        self.operations.publish_plan_activity(
            request.session,
            request.plan,
            None,
            scriberig_session::SessionActivityKind::PlanRevised,
            scriberig_session::SessionActivityState::Completed,
            "Plan revised",
        );
        Ok(updated)
    }

    /// Validates and stores a structured plan proposal without starting it.
    ///
    /// - `self`: Application service owning durable session mutations.
    /// - `context`: Authorized caller context.
    /// - `request`: Session, exact anchor, goal, nodes, and dependency indexes.
    ///
    /// The authoritative plan with application-allocated identities.
    ///
    /// Returns validation, authorization, anchor, or persistence failures.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn create_plan(
        &self,
        context: &ApplicationContext,
        request: &PlanCreateRequest,
    ) -> Result<Plan, ApplicationError> {
        validate_proposal(request)?;
        let mut session = self.read_session(
            context,
            &crate::SessionRequest {
                session: request.session,
            },
        )?;
        let anchor = session
            .resolve_anchor(request.anchor)
            .map_err(|error| invalid_plan(error.to_string()))?;
        let mut plan = Plan::new(
            self.next_entity(0xC1),
            request.session,
            request.anchor,
            request.goal.clone(),
        )
        .map_err(|error| invalid_plan(format!("invalid plan: {error:?}")))?;
        plan.transition(PlanStatus::Planned)
            .map_err(|error| invalid_plan(format!("invalid plan state: {error:?}")))?;
        let node_ids = request
            .nodes
            .iter()
            .map(|_| self.next_entity(0xC2))
            .collect::<Vec<_>>();
        for (index, draft) in request.nodes.iter().enumerate() {
            if let Some(assignment) = &draft.assignment {
                validate_assignment(&self.model_library, assignment)?;
            }
            let parent = draft
                .parent_index
                .map_or(Some(PlanNodeParent::Branch(anchor.branch)), |parent| {
                    Some(PlanNodeParent::Node(node_ids[parent]))
                });
            plan.add_node(PlanNode {
                id: node_ids[index],
                parent,
                title: draft.title.clone(),
                purpose: draft.purpose.clone(),
                expected_result: draft.expected_result.clone(),
                semantic_scope: draft.semantic_scope.clone(),
                status: PlanNodeStatus::Planned,
                branch: None,
                result: None,
                superseded_by: None,
                assignment: draft.assignment.clone(),
                claim: None,
                claim_generation: 0,
            })
            .map_err(|error| invalid_plan(format!("invalid plan node: {error:?}")))?;
        }
        for (prerequisite, dependent) in &request.dependencies {
            plan.add_dependency(node_ids[*prerequisite], node_ids[*dependent])
                .map_err(|error| invalid_plan(format!("invalid dependency: {error:?}")))?;
        }
        session
            .add_plan(plan.clone())
            .map_err(|error| invalid_plan(format!("invalid session plan: {error:?}")))?;
        self.save_session(context, &session)?;
        self.operations.publish_plan_activity(
            request.session,
            plan.id,
            None,
            scriberig_session::SessionActivityKind::PlanCreated,
            scriberig_session::SessionActivityState::Completed,
            "Plan created",
        );
        Ok(plan)
    }

    /// Changes one unstarted node assignment using the configured model library.
    ///
    /// - `self`: Application service owning plan and model-library state.
    /// - `context`: Authorized caller context.
    /// - `request`: Plan revision, target node, and safe assignment.
    ///
    /// The authoritative updated plan.
    ///
    /// Returns conflict, unsupported model configuration, invalid node state,
    /// lookup, or persistence failures.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn set_plan_node_assignment(
        &self,
        context: &ApplicationContext,
        request: &crate::PlanAssignmentRequest,
    ) -> Result<Plan, ApplicationError> {
        if let Some(assignment) = &request.assignment {
            validate_assignment(&self.model_library, assignment)?;
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
        plan.set_assignment(request.node, request.assignment.clone())
            .map_err(|error| invalid_plan(format!("invalid node assignment: {error:?}")))?;
        let updated = plan.clone();
        self.compare_and_swap_plan(context, &session, request.plan, request.expected_revision)?;
        self.operations.publish_plan_activity(
            request.session,
            request.plan,
            Some(request.node),
            scriberig_session::SessionActivityKind::AssignmentChanged,
            scriberig_session::SessionActivityState::Completed,
            "Plan node assignment changed",
        );
        Ok(updated)
    }
}
