//! Implements root agent session completion operations.

use super::super::persist_execution_evidence;
use crate::{ApplicationError, ApplicationService};
use scriberig_session::{BranchResult, Turn};

impl<B> ApplicationService<B> {
    /// Commits final runtime output into a pending local agent operation.
    ///
    /// - `self`: Application service owning local session state.
    /// - `handle`: Pending operation and branch handle.
    /// - `final_output`: Complete externally meaningful agent output.
    ///
    /// The structured application agent result stored on the operation.
    ///
    /// Returns an application error when the branch or operation is unavailable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn complete_local_agent(
        &self,
        handle: &crate::LocalAgentHandle,
        final_output: impl Into<String>,
    ) -> Result<crate::AgentResult, ApplicationError> {
        self.complete_local_agent_with_handles(handle, final_output, Vec::new())
    }

    /// Commits final output and stable result handles into a local agent branch.
    ///
    /// - `self`: Application service owning local session state.
    /// - `handle`: Pending operation and branch handle.
    /// - `final_output`: Complete externally meaningful agent output.
    /// - `result_handles`: Stable project, snapshot, manifest, or candidate handles.
    ///
    /// The structured application agent result stored on the operation.
    ///
    /// Returns an application error when the branch or operation is unavailable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn complete_local_agent_with_handles(
        &self,
        handle: &crate::LocalAgentHandle,
        final_output: impl Into<String>,
        result_handles: Vec<String>,
    ) -> Result<crate::AgentResult, ApplicationError> {
        self.complete_local_agent_with_evidence(
            handle,
            final_output,
            result_handles,
            Vec::new(),
            Vec::new(),
        )
    }

    /// Completes a root branch and persists its executed tool evidence.
    ///
    /// - `self`: Application service owning local session state.
    /// - `handle`: Pending operation and branch handle.
    /// - `final_output`: Complete externally meaningful agent output.
    /// - `result_handles`: Stable semantic references returned by the agent.
    /// - `tool_evidence`: Exact tool arguments and bounded structured results.
    /// - `model_requests`: Safe normalized requests captured by the runtime.
    ///
    /// The structured application agent result stored on the operation.
    ///
    /// Returns an application error when the branch or operation is unavailable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn complete_local_agent_with_evidence(
        &self,
        handle: &crate::LocalAgentHandle,
        final_output: impl Into<String>,
        result_handles: Vec<String>,
        tool_evidence: Vec<crate::ToolExecutionEvidence>,
        model_requests: Vec<scriberig_session::ModelRequestProjection>,
    ) -> Result<crate::AgentResult, ApplicationError> {
        let final_output = final_output.into();
        let mut session =
            self.load_session_state(handle.session)?
                .ok_or_else(|| ApplicationError::NotFound {
                    subject: handle.session.to_string(),
                })?;
        let branch = session
            .branches
            .iter_mut()
            .find(|branch| branch.id == handle.root_branch)
            .ok_or_else(|| ApplicationError::NotFound {
                subject: handle.root_branch.to_string(),
            })?;
        persist_execution_evidence(self, branch, tool_evidence, model_requests);
        let output_turn = if let Some(turn) = branch.turns.iter_mut().rev().find(|turn| {
            turn.agent.is_none()
                && turn
                    .human
                    .as_ref()
                    .is_some_and(|human| human.text == handle.prompt)
        }) {
            turn.agent = Some(scriberig_session::AgentOutput {
                text: final_output.clone(),
            });
            turn.id
        } else {
            let turn_id = self.next_entity(0xB2);
            branch.push_turn(Turn {
                id: turn_id,
                ordinal: 0,
                human: Some(scriberig_session::HumanInput {
                    text: handle.prompt.clone(),
                }),
                agent: Some(scriberig_session::AgentOutput {
                    text: final_output.clone(),
                }),
                tool_invocation: None,
                tool_result: None,
            });
            turn_id
        };
        branch
            .activity
            .push(scriberig_session::ExecutionActivity::AgentOutput(
                output_turn,
            ));
        let branch_result = self.next_entity(0xB3);
        branch.result = Some(BranchResult {
            id: branch_result,
            branch: branch.id,
            status: scriberig_session::BranchStatus::Completed,
            summary: Some(final_output.clone()),
            result_kind: Some("agent".to_owned()),
            candidate_context: None,
            references: Vec::new(),
            result_handles: result_handles.clone(),
            touched: Vec::new(),
            produced_changes: Vec::new(),
            validations: Vec::new(),
            unresolved_questions: Vec::new(),
        });
        branch
            .activity
            .push(scriberig_session::ExecutionActivity::BranchResult(
                branch_result,
            ));
        branch
            .transition(scriberig_session::BranchStatus::Completed)
            .map_err(|error| ApplicationError::Internal {
                message: error.to_string(),
            })?;
        if self.operations.get(&handle.operation)?.status == crate::OperationStatus::Cancelled {
            self.fail_local_agent(handle, "agent operation cancelled")?;
            return Err(ApplicationError::OperationFailed {
                operation: handle.operation.to_string(),
                message: "agent operation cancelled".to_owned(),
            });
        }
        self.save_session_state(session)?;
        let result = crate::AgentResult {
            session: handle.session,
            root_branch: handle.root_branch,
            final_output,
            branch_result,
            produced_changes: Vec::new(),
            validation_results: Vec::new(),
            unresolved_questions: Vec::new(),
            candidate_changes: Vec::new(),
        };
        self.operations
            .publish_agent_output(&handle.operation, &result.final_output)?;
        self.operations.complete_agent(
            &handle.operation,
            std::iter::once(branch_result.to_string())
                .chain(result_handles)
                .collect(),
            result.clone(),
        )?;
        Ok(result)
    }

    /// Accepts an agent request in the provider-neutral local session store.
    ///
    /// - `crate`: Root agent request containing the user prompt.
    /// - `self`: Application service owning session state.
    /// - `request`: Session, branch, and prompt supplied by the caller.
    ///
    /// A lifecycle operation identifier accepted for execution.
    ///
    /// Returns an application error when session state cannot be updated.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(crate) fn accept_local_agent(
        &self,
        request: &crate::AgentCommandRequest,
    ) -> Result<scriberig_core::OperationId, ApplicationError> {
        let handle = self.start_local_agent(request)?;
        self.complete_local_agent(&handle, request.prompt.clone())?;
        Ok(handle.operation)
    }
}
