//! Applies shared summary, evidence, gate, and waiver mutations.

use crate::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationService,
    PlanSemanticMutation, PlanSemanticMutationRequest, SessionBackend,
};
use scriberig_core::EntityId;
use scriberig_session::{Plan, PlanEvidenceAssociation, PlanGate, PlanGateWaiver, PlanNodeSummary};

use super::validation::{invalid_plan, revision_conflict};

impl<B: ApplicationBackend> ApplicationService<B> {
    /// Applies one typed semantic mutation at an exact plan revision.
    ///
    /// Actor provenance is derived from the trusted application context.
    ///
    /// # Parameters
    /// - `self`: Application service owning plan persistence.
    /// - `context`: Authorized caller context.
    /// - `request`: Plan revision and typed summary/evidence/gate mutation.
    ///
    /// The authoritative updated plan.
    ///
    /// Returns validation, conflict, lookup, or persistence failures.
    /// # Returns
    /// The updated plan.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn mutate_plan_semantics(
        &self,
        context: &ApplicationContext,
        request: &PlanSemanticMutationRequest,
    ) -> Result<Plan, ApplicationError> {
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
        let actor = context
            .principal
            .as_ref()
            .and_then(|principal| principal.id.parse::<EntityId>().ok());
        match request.mutation.clone() {
            PlanSemanticMutation::UpdateSummary {
                node,
                text,
                sources,
            } => plan.update_summary(PlanNodeSummary {
                id: self.next_entity(0xC8),
                node,
                text,
                sources,
            }),
            PlanSemanticMutation::AttachEvidence {
                owner,
                role,
                evidence,
                note,
            } => plan.attach_evidence(PlanEvidenceAssociation {
                id: self.next_entity(0xC9),
                owner,
                role,
                evidence,
                note,
                actor,
            }),
            PlanSemanticMutation::AddGate {
                owner,
                phase,
                requirement,
            } => plan.add_gate(PlanGate {
                id: self.next_entity(0xCA),
                owner,
                phase,
                requirement,
                waiver: None,
            }),
            PlanSemanticMutation::WaiveGate { gate, reason } => plan.waive_gate(
                gate,
                PlanGateWaiver {
                    reason,
                    actor,
                    operation: None,
                },
            ),
        }
        .map_err(|error| invalid_plan(format!("invalid semantic plan mutation: {error:?}")))?;
        let updated = plan.clone();
        self.compare_and_swap_plan(context, &session, request.plan, request.expected_revision)?;
        self.operations.publish_plan_activity(
            request.session,
            request.plan,
            None,
            scriberig_session::SessionActivityKind::PlanRevised,
            scriberig_session::SessionActivityState::Completed,
            "Plan semantics updated",
        );
        Ok(updated)
    }
}
